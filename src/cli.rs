//! CLI 인자 파싱

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "diffy",
    version,
    about = "Interactive TUI diff reviewer for Claude Code",
    long_about = "Review code changes hunk-by-hunk with an interactive TUI. Accept or reject individual hunks, add comments, and get structured feedback for Claude Code integration."
)]
pub struct Cli {
    /// Diff range (mutually exclusive)
    #[command(flatten)]
    pub diff_range: DiffRange,

    /// Claude Code hook mode: sends structured feedback to stderr (exit 0 on accept all, exit 2 if rejected)
    #[arg(long)]
    pub hook_mode: bool,

    /// Auto-apply: automatically revert rejected hunks to working tree
    #[arg(long)]
    pub apply: bool,

    /// Restore the last backup created by --apply
    #[arg(long)]
    pub restore: bool,

    /// Output as JSON instead of unified diff
    #[arg(long)]
    pub json: bool,

    /// Filter changes to specific path (optional, e.g., diffy -- src/main.rs)
    pub path: Option<String>,
}

#[derive(clap::Args, Debug)]
#[group(multiple = false)]
pub struct DiffRange {
    /// Review staged changes (git index)
    #[arg(long)]
    pub staged: bool,

    /// Review HEAD commit changes
    #[arg(long)]
    pub head: bool,

    /// Review changes against specific git ref (branch, tag, commit)
    #[arg(long = "ref", value_name = "REF")]
    pub git_ref: Option<String>,
}
