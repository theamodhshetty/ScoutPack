use crate::{context, index, search, token_budget};
use anyhow::{Context, Result};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

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
        Parameters(ContextRequest { task, budget }): Parameters<ContextRequest>,
    ) -> Result<CallToolResult, McpError> {
        let packet =
            context::build_context_packet(&self.root, &task, budget).map_err(to_mcp_error)?;
        Ok(structured(json!({
            "task": task,
            "budget": budget,
            "estimated_tokens": token_budget::estimate_tokens(&packet),
            "packet": packet,
        })))
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

fn structured(value: Value) -> CallToolResult {
    CallToolResult::structured(value)
}

fn to_mcp_error(error: anyhow::Error) -> McpError {
    McpError::internal_error(error.to_string(), None)
}
