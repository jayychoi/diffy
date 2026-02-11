//! diffy — 인터랙티브 diff 리뷰어
//!
//! 사용법: git diff | diffy | git apply

mod model;
mod output;
mod parse;
mod tty;
mod tui;

use std::io::{self, Read, Write};
use std::process;

use anyhow::Result;

fn run() -> Result<i32> {
    // stdin이 tty면 사용법 안내 출력 후 종료
    if tty::stdin_is_tty() {
        eprintln!("diffy — 인터랙티브 diff 리뷰어");
        eprintln!();
        eprintln!("사용법:");
        eprintln!("  git diff | diffy          diff를 리뷰하고 accepted 헌크만 stdout으로 출력");
        eprintln!("  git diff | diffy | git apply  리뷰 후 바로 적용");
        eprintln!();
        eprintln!("키바인딩:");
        eprintln!("  j/k     헌크 이동");
        eprintln!("  a/r     수락/거절");
        eprintln!("  A/R     전체 수락/거절");
        eprintln!("  Space   상태 토글");
        eprintln!("  q       종료");
        return Ok(0);
    }

    // stdin에서 diff 읽기
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let diff = parse::parse_diff(&input)?;

    if diff.files.is_empty() {
        eprintln!("[diffy] 변경사항이 없습니다.");
        return Ok(0);
    }

    let total_hunks: usize = diff.files.iter().map(|f| f.hunks.len()).sum();
    let reviewed_diff = tui::run(diff)?;

    let mut stdout = io::stdout().lock();
    let has_output = output::write_diff(&reviewed_diff, &mut stdout)?;
    stdout.flush()?;

    let accepted: usize = reviewed_diff
        .files
        .iter()
        .flat_map(|f| &f.hunks)
        .filter(|h| h.status == model::ReviewStatus::Accepted)
        .count();
    let rejected = total_hunks - accepted;

    eprintln!("[diffy] {accepted}/{total_hunks} hunks accepted, {rejected} rejected.");

    if has_output {
        Ok(0)
    } else {
        Ok(2)
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
