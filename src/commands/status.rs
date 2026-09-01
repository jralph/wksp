//! `wksp status [scope]`

use std::process::Command;

use anyhow::Result;

use crate::workspace_tree::{Repo, WorkspaceTree};

pub fn run(scope: Option<&str>) -> Result<()> {
    let tree = WorkspaceTree::discover()?;

    let repos: Vec<&Repo> = match scope {
        None => tree.repos.iter().collect(),
        Some(scope) => resolve_scope(&tree, scope),
    };

    if repos.is_empty() {
        println!("No repos matched.");
        return Ok(());
    }

    for repo in repos {
        let summary = repo_status(repo).unwrap_or_else(|e| format!("error: {e}"));
        println!(
            "{:<45} {:<25} {}",
            repo.org_slug(),
            repo.location(),
            summary
        );
    }

    Ok(())
}

fn resolve_scope<'a>(tree: &'a WorkspaceTree, scope: &str) -> Vec<&'a Repo> {
    if let Some((domain, workspace)) = scope.split_once('/') {
        tree.find_by_location(domain, Some(workspace))
    } else {
        tree.find_by_location(scope, None)
    }
}

fn repo_status(repo: &Repo) -> Result<String> {
    let dirty = has_uncommitted_changes(&repo.path)?;
    let (ahead, behind) = ahead_behind(&repo.path)?;

    let mut parts = Vec::new();
    parts.push(if dirty {
        "dirty".to_string()
    } else {
        "clean".to_string()
    });

    match (ahead, behind) {
        (Some(0), Some(0)) | (None, None) => {}
        (Some(a), Some(b)) => {
            if a > 0 {
                parts.push(format!("{a}\u{2191}"));
            }
            if b > 0 {
                parts.push(format!("{b}\u{2193}"));
            }
        }
        _ => parts.push("no upstream".to_string()),
    }

    Ok(parts.join(" "))
}

fn has_uncommitted_changes(path: &std::path::Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["status", "--porcelain"])
        .output()?;
    Ok(!output.stdout.is_empty())
}

/// Returns `(ahead, behind)` relative to the upstream, or `(None, None)` if
/// there is no upstream configured.
fn ahead_behind(path: &std::path::Path) -> Result<(Option<u32>, Option<u32>)> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
        .output()?;

    if !output.status.success() {
        // Most commonly: no upstream configured for the current branch.
        return Ok((None, None));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut counts = text.split_whitespace();
    let ahead = counts.next().and_then(|s| s.parse::<u32>().ok());
    let behind = counts.next().and_then(|s| s.parse::<u32>().ok());
    Ok((ahead, behind))
}
