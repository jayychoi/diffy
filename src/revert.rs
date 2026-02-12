//! 백업/복원/역방향 patch

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::git;
use crate::model::{Diff, DiffLine, ReviewStatus};

const MAX_BACKUP_REFS: usize = 10;

/// .diffy/ 디렉토리 경로 반환 (없으면 생성)
fn ensure_diffy_dir() -> Result<PathBuf> {
    let root = git::repo_root()?;
    let dir = root.join(".diffy");
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// git stash create → .diffy/backup-refs에 기록
pub fn backup() -> Result<String> {
    let output = Command::new("git")
        .args(["stash", "create"])
        .output()
        .context("git stash create failed")?;

    let sha = String::from_utf8(output.stdout)?.trim().to_string();
    if sha.is_empty() {
        // 변경사항 없으면 빈 문자열
        return Ok(String::new());
    }

    let dir = ensure_diffy_dir()?;
    let ref_file = dir.join("backup-refs");

    // append mode
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ref_file)?;
    writeln!(f, "{}", sha)?;

    prune_backups(&ref_file)?;

    Ok(sha)
}

/// Keep only the last MAX_BACKUP_REFS entries
fn prune_backups(ref_file: &std::path::Path) -> Result<()> {
    let contents = fs::read_to_string(ref_file)?;
    let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() > MAX_BACKUP_REFS {
        let kept = &lines[lines.len() - MAX_BACKUP_REFS..];
        fs::write(ref_file, kept.join("\n") + "\n")?;
    }
    Ok(())
}

/// 마지막 backup에서 git stash apply
pub fn restore() -> Result<i32> {
    let dir = ensure_diffy_dir()?;
    let ref_file = dir.join("backup-refs");

    if !ref_file.exists() {
        eprintln!("[diffy] No backup found.");
        return Ok(1);
    }

    let contents = fs::read_to_string(&ref_file)?;
    let last_ref = contents.lines().last().unwrap_or("").trim();

    if last_ref.is_empty() {
        eprintln!("[diffy] No backup found.");
        return Ok(1);
    }

    let status = Command::new("git")
        .args(["stash", "apply", last_ref])
        .status()
        .context("git stash apply failed")?;

    if status.success() {
        // Remove the restored ref from backup-refs
        let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() <= 1 {
            fs::remove_file(&ref_file)?;
        } else {
            let kept = &lines[..lines.len() - 1];
            fs::write(&ref_file, kept.join("\n") + "\n")?;
        }
        eprintln!(
            "[diffy] Backup restored: {}",
            &last_ref[..8.min(last_ref.len())]
        );
        Ok(0)
    } else {
        eprintln!("[diffy] Backup restore failed.");
        Ok(1)
    }
}

/// rejected 헌크로부터 역방향 patch 생성
pub fn generate_reverse_patch(diff: &Diff) -> String {
    let mut output = String::new();

    for file in &diff.files {
        if file.is_binary {
            continue;
        }

        let rejected_hunks: Vec<_> = file
            .hunks
            .iter()
            .filter(|h| h.status == ReviewStatus::Rejected)
            .collect();

        if rejected_hunks.is_empty() {
            continue;
        }

        output.push_str(&format!("--- {}\n", file.raw_new_path.replace("b/", "a/")));
        output.push_str(&format!("+++ {}\n", file.raw_new_path));

        for hunk in rejected_hunks {
            // 역방향: old ↔ new 교환
            let rev_header = format!(
                "@@ -{},{} +{},{} @@",
                hunk.new_start, hunk.new_count, hunk.old_start, hunk.old_count,
            );
            output.push_str(&rev_header);
            output.push('\n');

            for line in &hunk.lines {
                match line {
                    DiffLine::Context(s) => {
                        output.push(' ');
                        output.push_str(s);
                        output.push('\n');
                    }
                    DiffLine::Added(s) => {
                        // 역방향: Added → Removed
                        output.push('-');
                        output.push_str(s);
                        output.push('\n');
                    }
                    DiffLine::Removed(s) => {
                        // 역방향: Removed → Added
                        output.push('+');
                        output.push_str(s);
                        output.push('\n');
                    }
                    DiffLine::NoNewline => {
                        output.push_str("\\ No newline at end of file\n");
                    }
                }
            }
        }
    }

    output
}

/// git apply로 역방향 patch 적용
pub fn apply_reverse(patch: &str) -> Result<()> {
    use std::io::Write;
    let mut child = Command::new("git")
        .args(["apply", "--allow-empty"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("git apply failed to start")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(patch.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        bail!("git apply failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FileDiff, Hunk};

    fn make_hunk(
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
        lines: Vec<DiffLine>,
        status: ReviewStatus,
    ) -> Hunk {
        Hunk {
            header: format!(
                "@@ -{},{} +{},{} @@",
                old_start, old_count, new_start, new_count
            ),
            old_start,
            old_count,
            new_start,
            new_count,
            lines,
            status,
            comment: None,
        }
    }

    fn make_file(path: &str, hunks: Vec<Hunk>) -> FileDiff {
        FileDiff {
            old_path: path.to_string(),
            new_path: path.to_string(),
            raw_old_path: format!("a/{}", path),
            raw_new_path: format!("b/{}", path),
            hunks,
            is_binary: false,
        }
    }

    #[test]
    fn test_reverse_single_add() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![make_hunk(
                    1,
                    3,
                    1,
                    4,
                    vec![
                        DiffLine::Context("line1".to_string()),
                        DiffLine::Context("line2".to_string()),
                        DiffLine::Added("new line".to_string()),
                        DiffLine::Context("line3".to_string()),
                    ],
                    ReviewStatus::Rejected,
                )],
            )],
        };

        let patch = generate_reverse_patch(&diff);
        assert!(patch.contains("@@ -1,4 +1,3 @@"));
        assert!(patch.contains("-new line"));
        assert!(patch.contains(" line1"));
    }

    #[test]
    fn test_reverse_single_remove() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![make_hunk(
                    1,
                    4,
                    1,
                    3,
                    vec![
                        DiffLine::Context("line1".to_string()),
                        DiffLine::Removed("deleted line".to_string()),
                        DiffLine::Context("line2".to_string()),
                        DiffLine::Context("line3".to_string()),
                    ],
                    ReviewStatus::Rejected,
                )],
            )],
        };

        let patch = generate_reverse_patch(&diff);
        assert!(patch.contains("@@ -1,3 +1,4 @@"));
        assert!(patch.contains("+deleted line"));
    }

    #[test]
    fn test_reverse_mixed() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![make_hunk(
                    1,
                    3,
                    1,
                    3,
                    vec![
                        DiffLine::Context("line1".to_string()),
                        DiffLine::Removed("old".to_string()),
                        DiffLine::Added("new".to_string()),
                        DiffLine::Context("line3".to_string()),
                    ],
                    ReviewStatus::Rejected,
                )],
            )],
        };

        let patch = generate_reverse_patch(&diff);
        assert!(patch.contains("@@ -1,3 +1,3 @@"));
        assert!(patch.contains("-new"));
        assert!(patch.contains("+old"));
    }

    #[test]
    fn test_reverse_header_recalc() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![make_hunk(
                    10,
                    5,
                    10,
                    8,
                    vec![DiffLine::Added("a".to_string())],
                    ReviewStatus::Rejected,
                )],
            )],
        };

        let patch = generate_reverse_patch(&diff);
        // new→old, old→new 교환
        assert!(patch.contains("@@ -10,8 +10,5 @@"));
    }

    #[test]
    fn test_reverse_only_rejected() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![
                    make_hunk(
                        1,
                        2,
                        1,
                        3,
                        vec![DiffLine::Added("accepted".to_string())],
                        ReviewStatus::Accepted,
                    ),
                    make_hunk(
                        10,
                        2,
                        11,
                        3,
                        vec![DiffLine::Added("rejected".to_string())],
                        ReviewStatus::Rejected,
                    ),
                ],
            )],
        };

        let patch = generate_reverse_patch(&diff);
        assert!(!patch.contains("accepted"));
        assert!(patch.contains("-rejected"));
    }

    #[test]
    fn test_prune_under_limit_noop() {
        let dir = std::env::temp_dir().join("diffy_test_prune_under");
        let _ = fs::remove_file(&dir);
        let mut content = String::new();
        for i in 0..MAX_BACKUP_REFS {
            content.push_str(&format!("sha{}\n", i));
        }
        fs::write(&dir, &content).unwrap();
        prune_backups(&dir).unwrap();
        let result = fs::read_to_string(&dir).unwrap();
        let lines: Vec<_> = result.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), MAX_BACKUP_REFS);
        fs::remove_file(&dir).unwrap();
    }

    #[test]
    fn test_prune_over_limit() {
        let dir = std::env::temp_dir().join("diffy_test_prune_over");
        let _ = fs::remove_file(&dir);
        let mut content = String::new();
        for i in 0..15 {
            content.push_str(&format!("sha{}\n", i));
        }
        fs::write(&dir, &content).unwrap();
        prune_backups(&dir).unwrap();
        let result = fs::read_to_string(&dir).unwrap();
        let lines: Vec<_> = result.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), MAX_BACKUP_REFS);
        // Should keep the last 10 (sha5..sha14)
        assert_eq!(lines[0], "sha5");
        assert_eq!(lines[9], "sha14");
        fs::remove_file(&dir).unwrap();
    }

    #[test]
    fn test_reverse_empty_when_all_accepted() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![make_hunk(
                    1,
                    2,
                    1,
                    3,
                    vec![DiffLine::Added("line".to_string())],
                    ReviewStatus::Accepted,
                )],
            )],
        };

        let patch = generate_reverse_patch(&diff);
        assert!(patch.is_empty());
    }
}
