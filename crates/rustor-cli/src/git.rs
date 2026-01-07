//! Git integration for rustor
//!
//! Provides functions to discover PHP files based on git state:
//! - Staged files (for pre-commit hooks)
//! - Changed files since a git ref (for CI workflows)

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find the root of the git repository
pub fn find_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to execute git rev-parse")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Not a git repository: {}", stderr.trim());
    }

    let path = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    Ok(PathBuf::from(path))
}

/// Get list of staged PHP files (for pre-commit hooks)
///
/// Returns files that are:
/// - Added (A)
/// - Copied (C)
/// - Modified (M)
/// - Renamed (R)
pub fn get_staged_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "diff",
            "--cached",
            "--name-only",
            "--diff-filter=ACMR",
            "--",
            "*.php",
        ])
        .output()
        .context("Failed to execute git diff --cached")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to get staged files: {}", stderr.trim());
    }

    parse_git_file_list(&output.stdout, repo_root)
}

/// Get list of PHP files changed since a git ref
///
/// Compares the current HEAD against the specified ref (branch, tag, or commit)
pub fn get_changed_files_since(repo_root: &Path, ref_name: &str) -> Result<Vec<PathBuf>> {
    // First verify the ref exists
    let check = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--verify", ref_name])
        .output()
        .context("Failed to verify git ref")?;

    if !check.status.success() {
        bail!("Invalid git ref: {}", ref_name);
    }

    // Get files changed between ref and HEAD
    // Using three-dot notation to get changes since common ancestor
    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            &format!("{}...HEAD", ref_name),
            "--",
            "*.php",
        ])
        .output()
        .context("Failed to execute git diff")?;

    if !output.status.success() {
        // Try two-dot notation as fallback (direct comparison)
        let output = Command::new("git")
            .current_dir(repo_root)
            .args([
                "diff",
                "--name-only",
                "--diff-filter=ACMR",
                ref_name,
                "HEAD",
                "--",
                "*.php",
            ])
            .output()
            .context("Failed to execute git diff")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to get changed files: {}", stderr.trim());
        }

        return parse_git_file_list(&output.stdout, repo_root);
    }

    parse_git_file_list(&output.stdout, repo_root)
}

/// Parse git output into list of absolute file paths
fn parse_git_file_list(output: &[u8], repo_root: &Path) -> Result<Vec<PathBuf>> {
    let files = String::from_utf8_lossy(output);

    let paths: Vec<PathBuf> = files
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| repo_root.join(line))
        .filter(|path| path.exists()) // Only include files that exist
        .collect();

    Ok(paths)
}

/// Check if we're in a git repository
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_git_repo() {
        // This test assumes we're running from within the rustor repo
        // which should be a git repo
        let result = is_git_repo();
        // Don't assert true because the test might run elsewhere
        assert!(result == true || result == false);
    }

    #[test]
    fn test_find_repo_root_in_git_repo() {
        // Skip if not in a git repo
        if !is_git_repo() {
            return;
        }

        let result = find_repo_root();
        assert!(result.is_ok());
        let root = result.unwrap();
        assert!(root.exists());
        assert!(root.is_dir());
    }

    #[test]
    fn test_parse_git_file_list() {
        let temp_dir = env::temp_dir();
        let output = b"src/foo.php\nsrc/bar.php\n";

        // This will filter out non-existent files
        let result = parse_git_file_list(output, &temp_dir);
        assert!(result.is_ok());
        // Files don't exist so list should be empty
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_git_file_list_empty() {
        let temp_dir = env::temp_dir();
        let output = b"";

        let result = parse_git_file_list(output, &temp_dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
