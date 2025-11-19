use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use tokio::process::Command;
use tokio::time::timeout;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 2000;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

#[derive(Deserialize)]
pub struct GrepFilesArgs {
    pub pattern: String,
    #[serde(default)]
    pub include: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

pub async fn grep_files(
    pattern: &str,
    include: Option<&str>,
    search_path: &Path,
    limit: usize,
    cwd: &Path,
) -> anyhow::Result<Vec<String>> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        anyhow::bail!("pattern must not be empty");
    }

    if limit == 0 {
        anyhow::bail!("limit must be greater than zero");
    }

    let limit = limit.min(MAX_LIMIT);

    verify_path_exists(search_path).await?;

    let include = include.map(str::trim).and_then(|val| {
        if val.is_empty() {
            None
        } else {
            Some(val.to_string())
        }
    });

    run_rg_search(pattern, include.as_deref(), search_path, limit, cwd).await
}

async fn verify_path_exists(path: &Path) -> anyhow::Result<()> {
    tokio::fs::metadata(path)
        .await
        .map_err(|err| anyhow::anyhow!("unable to access `{}`: {err}", path.display()))?;
    Ok(())
}

async fn run_rg_search(
    pattern: &str,
    include: Option<&str>,
    search_path: &Path,
    limit: usize,
    cwd: &Path,
) -> anyhow::Result<Vec<String>> {
    let mut command = Command::new("rg");
    command
        .current_dir(cwd)
        .arg("--files-with-matches")
        .arg("--sortr=modified")
        .arg("--regexp")
        .arg(pattern)
        .arg("--no-messages");

    if let Some(glob) = include {
        command.arg("--glob").arg(glob);
    }

    command.arg("--").arg(search_path);

    let output = timeout(COMMAND_TIMEOUT, command.output())
        .await
        .map_err(|_| anyhow::anyhow!("rg timed out after 30 seconds"))?
        .map_err(|err| {
            anyhow::anyhow!(
                "failed to launch rg: {err}. Ensure ripgrep is installed and on PATH."
            )
        })?;

    match output.status.code() {
        Some(0) => Ok(parse_results(&output.stdout, limit)),
        Some(1) => Ok(Vec::new()),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("rg failed: {stderr}")
        }
    }
}

fn parse_results(stdout: &[u8], limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    for line in stdout.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        if let Ok(text) = std::str::from_utf8(line) {
            if text.is_empty() {
                continue;
            }
            results.push(text.to_string());
            if results.len() == limit {
                break;
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;

    #[test]
    fn parses_basic_results() {
        let stdout = b"/tmp/file_a.rs\n/tmp/file_b.rs\n";
        let parsed = parse_results(stdout, 10);
        assert_eq!(
            parsed,
            vec!["/tmp/file_a.rs".to_string(), "/tmp/file_b.rs".to_string()]
        );
    }

    #[test]
    fn parse_truncates_after_limit() {
        let stdout = b"/tmp/file_a.rs\n/tmp/file_b.rs\n/tmp/file_c.rs\n";
        let parsed = parse_results(stdout, 2);
        assert_eq!(
            parsed,
            vec!["/tmp/file_a.rs".to_string(), "/tmp/file_b.rs".to_string()]
        );
    }

    #[tokio::test]
    async fn run_search_returns_results() -> anyhow::Result<()> {
        if !rg_available() {
            return Ok(());
        }
        let temp = tempdir()?;
        let dir = temp.path();
        std::fs::write(dir.join("match_one.txt"), "alpha beta gamma")?;
        std::fs::write(dir.join("match_two.txt"), "alpha delta")?;
        std::fs::write(dir.join("other.txt"), "omega")?;

        let results = run_rg_search("alpha", None, dir, 10, dir).await?;
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|path| path.ends_with("match_one.txt")));
        assert!(results.iter().any(|path| path.ends_with("match_two.txt")));
        Ok(())
    }

    #[tokio::test]
    async fn run_search_with_glob_filter() -> anyhow::Result<()> {
        if !rg_available() {
            return Ok(());
        }
        let temp = tempdir()?;
        let dir = temp.path();
        std::fs::write(dir.join("match_one.rs"), "alpha beta gamma")?;
        std::fs::write(dir.join("match_two.txt"), "alpha delta")?;

        let results = run_rg_search("alpha", Some("*.rs"), dir, 10, dir).await?;
        assert_eq!(results.len(), 1);
        assert!(results.iter().all(|path| path.ends_with("match_one.rs")));
        Ok(())
    }

    #[tokio::test]
    async fn run_search_respects_limit() -> anyhow::Result<()> {
        if !rg_available() {
            return Ok(());
        }
        let temp = tempdir()?;
        let dir = temp.path();
        std::fs::write(dir.join("one.txt"), "alpha one")?;
        std::fs::write(dir.join("two.txt"), "alpha two")?;
        std::fs::write(dir.join("three.txt"), "alpha three")?;

        let results = run_rg_search("alpha", None, dir, 2, dir).await?;
        assert_eq!(results.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn run_search_handles_no_matches() -> anyhow::Result<()> {
        if !rg_available() {
            return Ok(());
        }
        let temp = tempdir()?;
        let dir = temp.path();
        std::fs::write(dir.join("one.txt"), "omega")?;

        let results = run_rg_search("alpha", None, dir, 5, dir).await?;
        assert!(results.is_empty());
        Ok(())
    }

    fn rg_available() -> bool {
        StdCommand::new("rg")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}
