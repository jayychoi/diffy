//! CLI 인자 파싱

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "diffy",
    version,
    about = "Interactive diff reviewer for Claude Code"
)]
pub struct Cli {
    /// Diff range (mutually exclusive)
    #[command(flatten)]
    pub diff_range: DiffRange,

    /// Claude Code hook mode (structured stderr feedback)
    #[arg(long)]
    pub hook_mode: bool,

    /// Actually apply revert for rejected hunks
    #[arg(long)]
    pub apply: bool,

    /// Restore last backup
    #[arg(long)]
    pub restore: bool,

    /// Output as JSON instead of unified diff
    #[arg(long)]
    pub json: bool,

    /// Target path filter
    pub path: Option<String>,
}

#[derive(clap::Args, Debug)]
#[group(multiple = false)]
pub struct DiffRange {
    /// Review staged changes
    #[arg(long)]
    pub staged: bool,

    /// Review HEAD changes
    #[arg(long)]
    pub head: bool,

    /// Review against specific ref
    #[arg(long = "ref", value_name = "REF")]
    pub git_ref: Option<String>,
}
