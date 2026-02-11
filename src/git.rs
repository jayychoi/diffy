//! Git CLI 래퍼

use std::path::PathBuf;
use std::process::Command;
use anyhow::{Result, bail, Context};

pub enum DiffMode {
    Unstaged,
    Staged,
    Head,
    Ref(String),
}

pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git not found")?;
    if !output.status.success() {
        bail!("git rev-parse failed");
    }
    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
}

pub fn git_diff(mode: &DiffMode, path: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("diff");
    match mode {
        DiffMode::Unstaged => {}
        DiffMode::Staged => { cmd.arg("--staged"); }
        DiffMode::Head => { cmd.arg("HEAD"); }
        DiffMode::Ref(r) => { cmd.arg(r); }
    }
    if let Some(p) = path {
        cmd.arg("--").arg(p);
    }
    let output = cmd.output().context("git not found")?;
    if !output.status.success() {
        bail!("git diff failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_repo() {
        // 이 테스트는 diffy 프로젝트 내에서 실행되므로 true여야 함
        assert!(is_git_repo());
    }

    #[test]
    fn test_repo_root() {
        let root = repo_root().unwrap();
        assert!(root.join("Cargo.toml").exists());
    }

    #[test]
    fn test_git_diff_unstaged() {
        // unstaged diff는 에러 없이 실행돼야 함
        let result = git_diff(&DiffMode::Unstaged, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_git_diff_staged() {
        let result = git_diff(&DiffMode::Staged, None);
        assert!(result.is_ok());
    }
}
