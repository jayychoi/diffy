//! diffy — 인터랙티브 diff 리뷰어
//!
//! 사용법:
//!   git diff | diffy | git apply    (파이프 모드)
//!   diffy [--staged|--head|--ref REF] [--json] [--hook-mode] [--apply]

mod cli;
mod git;
mod hook;
mod model;
mod output;
mod parse;
mod revert;
mod tty;
mod tui;

use std::io::{self, Read, Write};
use std::process;

use anyhow::Result;
use clap::Parser;

use cli::Cli;

fn run() -> Result<i32> {
    let cli = Cli::parse();

    // 분기 1: --restore
    if cli.restore {
        return revert::restore();
    }

    // 분기 2: stdin이 파이프 → 기존 파이프 모드 (후방 호환)
    if !tty::stdin_is_tty() {
        return run_pipe_mode(&cli);
    }

    // 분기 3: CLI 모드 → git diff 내부 실행
    run_cli_mode(&cli)
}

/// 파이프 모드: git diff | diffy | git apply
fn run_pipe_mode(cli: &Cli) -> Result<i32> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let diff = parse::parse_diff(&input)?;

    if diff.files.is_empty() {
        eprintln!("[diffy] No changes to review.");
        return Ok(0);
    }

    let total_hunks: usize = diff.files.iter().map(|f| f.hunks.len()).sum();
    let reviewed_diff = tui::run(diff)?;

    write_output(&reviewed_diff, cli, total_hunks)
}

/// CLI 모드: diffy --staged / diffy --head / diffy --ref REF
fn run_cli_mode(cli: &Cli) -> Result<i32> {
    if !git::is_git_repo() {
        eprintln!("[diffy] Not a git repository.");
        return Ok(1);
    }

    if !git::has_commits() {
        eprintln!("[diffy] No commits yet. Create an initial commit first.");
        return Ok(1);
    }

    let mode = resolve_diff_mode(cli);
    let diff_text = git::git_diff(&mode, cli.path.as_deref())?;

    if diff_text.is_empty() {
        eprintln!("[diffy] No changes to review.");
        return Ok(0);
    }

    let diff = parse::parse_diff(&diff_text)?;

    if diff.files.is_empty() {
        eprintln!("[diffy] No changes to review.");
        return Ok(0);
    }

    let total_hunks: usize = diff.files.iter().map(|f| f.hunks.len()).sum();

    // --apply: backup before review
    if cli.apply {
        revert::backup()?;
    }

    let reviewed_diff = tui::run(diff)?;

    // --apply: rejected 헌크 되돌리기
    if cli.apply {
        let reverse = revert::generate_reverse_patch(&reviewed_diff);
        if !reverse.is_empty() {
            revert::apply_reverse(&reverse)?;
        }
    }

    // --hook-mode: stderr 피드백
    if cli.hook_mode {
        let all_accepted = hook::write_feedback(&reviewed_diff, &mut io::stderr())?;
        return if all_accepted { Ok(0) } else { Ok(2) };
    }

    write_output(&reviewed_diff, cli, total_hunks)
}

/// 리뷰 결과 출력 (diff 또는 JSON)
fn write_output(diff: &model::Diff, cli: &Cli, total_hunks: usize) -> Result<i32> {
    let mut stdout = io::stdout().lock();

    if cli.json {
        output::write_json(diff, &mut stdout)?;
        stdout.flush()?;
        return Ok(0);
    }

    let has_output = output::write_diff(diff, &mut stdout)?;
    stdout.flush()?;

    let accepted: usize = diff.files.iter()
        .flat_map(|f| &f.hunks)
        .filter(|h| h.status == model::ReviewStatus::Accepted)
        .count();
    let rejected = total_hunks - accepted;

    eprintln!("[diffy] {accepted}/{total_hunks} hunks accepted, {rejected} rejected.");

    if has_output { Ok(0) } else { Ok(2) }
}

fn resolve_diff_mode(cli: &Cli) -> git::DiffMode {
    if cli.diff_range.staged {
        git::DiffMode::Staged
    } else if cli.diff_range.head {
        git::DiffMode::Head
    } else if let Some(ref r) = cli.diff_range.git_ref {
        git::DiffMode::Ref(r.clone())
    } else {
        git::DiffMode::Unstaged
    }
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("[diffy] Error: {e:#}");
            process::exit(1);
        }
    }
}
