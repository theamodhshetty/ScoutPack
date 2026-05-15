use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "scoutpack")]
#[command(version)]
#[command(about = "Offline repo context for AI agents.")]
pub struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create scoutpack.toml and .scoutpackignore.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Scan a repo/folder and build a local ScoutPack index.
    Pack {
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Generate local semantic embeddings for chunks. Requires `--features semantic`.
        #[arg(long, default_value_t = false)]
        embed: bool,

        /// Allow one-time local model download for `--embed` without an interactive prompt.
        #[arg(long, default_value_t = false)]
        allow_model_download: bool,
    },

    /// Watch a repo/folder and refresh the local index after file changes.
    Watch {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long, default_value_t = 500)]
        debounce_ms: u64,
    },

    /// Search the local ScoutPack index.
    Search {
        query: String,

        #[arg(short, long, default_value_t = 5)]
        limit: usize,

        #[arg(long, default_value_t = false)]
        show_snippets: bool,

        #[arg(long, default_value_t = false)]
        json: bool,

        /// Use local semantic embeddings. Requires an index created with `pack --embed`.
        #[arg(long, default_value_t = false)]
        semantic: bool,

        /// Weight for semantic score in hybrid ranking.
        #[arg(long, default_value_t = 0.45)]
        semantic_alpha: f64,
    },

    /// Build a compact task-specific markdown context packet.
    Context {
        task: String,

        #[arg(short, long)]
        budget: Option<usize>,

        #[arg(long, value_enum, default_value_t = ContextFormat::Markdown)]
        format: ContextFormat,

        #[arg(long, default_value_t = false)]
        json: bool,

        /// Boost files changed since this git ref.
        #[arg(long)]
        since: Option<String>,

        /// Scope context to files changed in a git diff range, for example main..HEAD.
        #[arg(long)]
        diff: Option<String>,

        /// Scope context to files changed on the current branch against main.
        #[arg(long, default_value_t = false)]
        branch: bool,

        /// Use local semantic embeddings. Requires an index created with `pack --embed`.
        #[arg(long, default_value_t = false)]
        semantic: bool,

        /// Weight for semantic score in hybrid ranking.
        #[arg(long, default_value_t = 0.45)]
        semantic_alpha: f64,

        /// Follow direct symbol calls from matched symbols by N hops.
        #[arg(long, default_value_t = 0)]
        expand_calls: usize,
    },

    /// Render an agent-ready prompt from a named template and ScoutPack context.
    Template {
        name: String,

        task: String,

        #[arg(short, long)]
        budget: Option<usize>,

        #[arg(long, default_value_t = false)]
        json: bool,

        /// Boost files changed since this git ref.
        #[arg(long)]
        since: Option<String>,

        /// Scope context to files changed in a git diff range, for example main..HEAD.
        #[arg(long)]
        diff: Option<String>,

        /// Scope context to files changed on the current branch against main.
        #[arg(long, default_value_t = false)]
        branch: bool,
    },

    /// Show index stats.
    Stats {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Start a read-only MCP server over an existing ScoutPack index.
    Mcp {
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Serve MCP over Streamable HTTP/SSE instead of stdio.
        #[arg(long, default_value_t = false)]
        http: bool,

        /// Host to bind for HTTP MCP mode.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind for HTTP MCP mode.
        #[arg(long, default_value_t = 7777)]
        port: u16,
    },

    /// Generate shell completion script.
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ContextFormat {
    Markdown,
    Json,
    Xml,
}
