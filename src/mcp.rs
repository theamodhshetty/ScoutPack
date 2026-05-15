use crate::{context, git, index, search, templates, token_budget};
use anyhow::{Context, Result};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct ScoutpackMcp {
    root: PathBuf,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl ScoutpackMcp {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchRequest {
    #[schemars(description = "Task or code search query.")]
    query: String,
    #[schemars(description = "Maximum ranked results to return. Defaults to 5.")]
    limit: Option<usize>,
    #[schemars(description = "Include compact source snippets in each result.")]
    show_snippets: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ContextRequest {
    #[schemars(description = "AI coding task to build context for.")]
    task: String,
    #[schemars(description = "Approximate token budget for the returned packet.")]
    budget: Option<usize>,
    #[schemars(description = "Follow direct symbol calls by N hops. Defaults to 0.")]
    expand_calls: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TemplateRequest {
    #[schemars(
        description = "Template name, for example bugfix, refactor, review, docs, or test."
    )]
    name: String,
    #[schemars(description = "AI coding task to build an agent-ready prompt for.")]
    task: String,
    #[schemars(description = "Approximate token budget for the embedded context packet.")]
    budget: Option<usize>,
    #[schemars(description = "Boost/summarize files changed since this git ref.")]
    since: Option<String>,
    #[schemars(description = "Git diff range like main..HEAD.")]
    diff: Option<String>,
    #[schemars(description = "Use current branch against main.")]
    branch: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FileSummaryRequest {
    #[schemars(description = "Indexed repository-relative file path.")]
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SymbolRequest {
    #[schemars(description = "Symbol name or partial symbol name.")]
    query: String,
    #[schemars(description = "Maximum symbol matches to return. Defaults to 10.")]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RecentChangesRequest {
    #[schemars(description = "Boost/summarize files changed since this git ref.")]
    since: Option<String>,
    #[schemars(description = "Git diff range like main..HEAD.")]
    diff: Option<String>,
    #[schemars(description = "Use current branch against main.")]
    branch: Option<bool>,
}

#[tool_router]
impl ScoutpackMcp {
    #[tool(
        name = "search",
        description = "Search the existing local ScoutPack index and return ranked code context."
    )]
    fn search(
        &self,
        Parameters(SearchRequest {
            query,
            limit,
            show_snippets,
        }): Parameters<SearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit.unwrap_or(5).min(50);
        let results =
            search::search_repo(&self.root, &query, limit, show_snippets.unwrap_or(false))
                .map_err(to_mcp_error)?;
        Ok(structured(json!({
            "query": query,
            "limit": limit,
            "results": results,
        })))
    }

    #[tool(
        name = "context",
        description = "Build a compact task-specific markdown context packet from the local index."
    )]
    fn context(
        &self,
        Parameters(ContextRequest {
            task,
            budget,
            expand_calls,
        }): Parameters<ContextRequest>,
    ) -> Result<CallToolResult, McpError> {
        let expand_calls = expand_calls.unwrap_or(0);
        let packet = if expand_calls == 0 {
            context::build_context_packet(&self.root, &task, budget)
        } else {
            context::build_context_packet_with_options(
                &self.root,
                &task,
                budget,
                context::ContextOptions {
                    expand_calls,
                    ..context::ContextOptions::default()
                },
            )
        }
        .map_err(to_mcp_error)?;
        Ok(structured(json!({
            "task": task,
            "budget": budget,
            "estimated_tokens": token_budget::estimate_tokens(&packet),
            "packet": packet,
        })))
    }

    #[tool(
        name = "template",
        description = "Render an agent-ready prompt from a named ScoutPack template and local context."
    )]
    fn template(
        &self,
        Parameters(TemplateRequest {
            name,
            task,
            budget,
            since,
            diff,
            branch,
        }): Parameters<TemplateRequest>,
    ) -> Result<CallToolResult, McpError> {
        let git_mode =
            optional_mcp_git_mode(since, diff, branch.unwrap_or(false)).map_err(to_mcp_error)?;
        let prompt = templates::render_prompt(
            &self.root,
            &name,
            &task,
            templates::TemplateOptions { budget, git_mode },
        )
        .map_err(to_mcp_error)?;
        Ok(structured(templates::render_summary(
            &task, &prompt, &name, budget,
        )))
    }

    #[tool(
        name = "file_summary",
        description = "Return indexed metadata, symbols, and chunk ranges for one repository file."
    )]
    fn file_summary(
        &self,
        Parameters(FileSummaryRequest { path }): Parameters<FileSummaryRequest>,
    ) -> Result<CallToolResult, McpError> {
        let conn = index::ensure_index(&self.root).map_err(to_mcp_error)?;
        let summary = index::read_file_summary(&conn, &path).map_err(to_mcp_error)?;
        Ok(structured(json!({
            "path": path,
            "found": summary.is_some(),
            "summary": summary,
        })))
    }

    #[tool(
        name = "symbol",
        description = "Find indexed symbols by exact or partial name."
    )]
    fn symbol(
        &self,
        Parameters(SymbolRequest { query, limit }): Parameters<SymbolRequest>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit.unwrap_or(10).min(50);
        let conn = index::ensure_index(&self.root).map_err(to_mcp_error)?;
        let matches = index::find_symbols(&conn, &query, limit).map_err(to_mcp_error)?;
        Ok(structured(json!({
            "query": query,
            "limit": limit,
            "matches": matches,
        })))
    }

    #[tool(
        name = "commands",
        description = "Return package commands discovered during indexing without executing them."
    )]
    fn commands(&self) -> Result<CallToolResult, McpError> {
        let conn = index::ensure_index(&self.root).map_err(to_mcp_error)?;
        let commands = index::read_indexed_commands(&conn).map_err(to_mcp_error)?;
        Ok(structured(json!({
            "commands": commands,
        })))
    }

    #[tool(
        name = "recent_changes",
        description = "Return local git changed files and line counts for a ref range without reading full diff bodies."
    )]
    fn recent_changes(
        &self,
        Parameters(RecentChangesRequest {
            since,
            diff,
            branch,
        }): Parameters<RecentChangesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mode = mcp_git_mode(since, diff, branch.unwrap_or(false)).map_err(to_mcp_error)?;
        let changes = git::recent_changes(&self.root, &mode).map_err(to_mcp_error)?;
        Ok(structured(json!({
            "recent_changes": changes,
        })))
    }

    #[tool(
        name = "stats",
        description = "Return index counts and manifest metadata for the configured repo."
    )]
    fn stats(&self) -> Result<CallToolResult, McpError> {
        let stats = index::read_stats(&self.root).map_err(to_mcp_error)?;
        Ok(structured(json!({
            "index_path": stats.index_path,
            "file_count": stats.file_count,
            "chunk_count": stats.chunk_count,
            "symbol_count": stats.symbol_count,
            "command_count": stats.command_count,
            "embedding_count": stats.embedding_count,
            "skipped_count": stats.skipped_count,
            "manifest": stats.manifest,
        })))
    }
}

#[tool_handler]
impl ServerHandler for ScoutpackMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "scoutpack",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "ScoutPack is a read-only local repo context server. Run `scoutpack pack .` before using MCP tools.",
            )
    }
}

pub async fn serve(path: PathBuf) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }
    let root = path
        .canonicalize()
        .with_context(|| format!("Could not resolve path {}", path.display()))?;
    let service = ScoutpackMcp::new(root)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|err| anyhow::anyhow!(err))?;
    service.waiting().await?;
    Ok(())
}

pub async fn serve_http(path: PathBuf, host: &str, port: u16) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };

    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }
    let root = path
        .canonicalize()
        .with_context(|| format!("Could not resolve path {}", path.display()))?;
    let cancellation = CancellationToken::new();
    let service_root = root.clone();
    let service = StreamableHttpService::new(
        move || Ok(ScoutpackMcp::new(service_root.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(cancellation.child_token()),
    );
    let app = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .with_context(|| format!("Could not bind MCP HTTP server to {host}:{port}"))?;
    let addr = listener.local_addr()?;
    eprintln!(
        "ScoutPack MCP HTTP listening on http://{addr}/mcp for {}",
        root.display()
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancellation.cancel();
            }
        })
        .await
        .map_err(|err| anyhow::anyhow!(err))?;
    Ok(())
}

fn structured(value: Value) -> CallToolResult {
    CallToolResult::structured(value)
}

fn to_mcp_error(error: anyhow::Error) -> McpError {
    McpError::internal_error(error.to_string(), None)
}

fn mcp_git_mode(
    since: Option<String>,
    diff: Option<String>,
    branch: bool,
) -> Result<git::GitContextMode> {
    let selected = since.is_some() as u8 + diff.is_some() as u8 + branch as u8;
    if selected > 1 {
        anyhow::bail!("Use only one of `since`, `diff`, or `branch`.");
    }
    if let Some(since) = since {
        return Ok(git::GitContextMode::Since(since));
    }
    if let Some(diff) = diff {
        let Some((base, head)) = diff.split_once("..") else {
            anyhow::bail!("`diff` must use `<base>..<head>`, for example `main..HEAD`.");
        };
        if base.is_empty() || head.is_empty() {
            anyhow::bail!("`diff` must use `<base>..<head>`, for example `main..HEAD`.");
        }
        return Ok(git::GitContextMode::Diff {
            base: base.to_owned(),
            head: head.to_owned(),
        });
    }
    Ok(git::GitContextMode::Branch)
}

fn optional_mcp_git_mode(
    since: Option<String>,
    diff: Option<String>,
    branch: bool,
) -> Result<Option<git::GitContextMode>> {
    let selected = since.is_some() as u8 + diff.is_some() as u8 + branch as u8;
    if selected > 1 {
        anyhow::bail!("Use only one of `since`, `diff`, or `branch`.");
    }
    if selected == 0 {
        return Ok(None);
    }
    mcp_git_mode(since, diff, branch).map(Some)
}
