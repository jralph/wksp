//! `wksp doctor`

use std::fs;

use anyhow::Result;

use crate::workspace_tree::{WorkspaceTree, WORKTREES_DIR};

pub fn run() -> Result<()> {
    let tree = WorkspaceTree::discover()?;
    let mut issues: Vec<String> = Vec::new();

    if !tree.root.exists() {
        println!("{} does not exist; nothing to check.", tree.root.display());
        return Ok(());
    }

    check_domains(&tree, &mut issues)?;
    check_unattributed_repos(&tree, &mut issues);

    if issues.is_empty() {
        println!("No issues found under {}.", tree.root.display());
    } else {
        println!("Found {} issue(s):", issues.len());
        for issue in &issues {
            println!("  - {issue}");
        }
    }

    Ok(())
}

fn check_domains(tree: &WorkspaceTree, issues: &mut Vec<String>) -> Result<()> {
    for domain_entry in fs::read_dir(&tree.root)? {
        let domain_entry = domain_entry?;
        let domain_path = domain_entry.path();
        if !domain_path.is_dir() || is_hidden(&domain_path) {
            continue;
        }
        let domain_name = domain_entry.file_name().to_string_lossy().to_string();

        for entry in fs::read_dir(&domain_path)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_file() {
                if name != "AGENTS.md" && name != "README.md" {
                    issues.push(format!(
                        "{}: unexpected file directly under a domain (expected only AGENTS.md/README.md)",
                        path.display()
                    ));
                }
                continue;
            }

            if is_hidden(&path) {
                continue;
            }

            check_workspace(tree, &domain_name, &name, &path, issues)?;
        }
    }
    Ok(())
}

fn check_workspace(
    tree: &WorkspaceTree,
    domain_name: &str,
    workspace_name: &str,
    workspace_path: &std::path::Path,
    issues: &mut Vec<String>,
) -> Result<()> {
    let repo_count = tree
        .find_by_location(domain_name, Some(workspace_name))
        .len();
    let has_agents_md = workspace_path.join("AGENTS.md").exists();

    if repo_count == 0 && !has_agents_md {
        issues.push(format!(
            "{}: empty workspace (no repos, no AGENTS.md)",
            workspace_path.display()
        ));
    }

    let worktrees_path = workspace_path.join(WORKTREES_DIR);
    if worktrees_path.is_dir() {
        check_worktrees_dir(&worktrees_path, issues)?;
    }

    Ok(())
}

/// `.worktrees/<repo>/<worktree>` entries should themselves be git worktrees
/// (their `.git` is a file pointing back at the primary checkout), not plain
/// directories that snuck in.
fn check_worktrees_dir(worktrees_path: &std::path::Path, issues: &mut Vec<String>) -> Result<()> {
    for repo_entry in fs::read_dir(worktrees_path)? {
        let repo_entry = repo_entry?;
        let repo_dir = repo_entry.path();
        if !repo_dir.is_dir() {
            issues.push(format!(
                "{}: expected only per-repo directories inside .worktrees",
                repo_dir.display()
            ));
            continue;
        }

        for worktree_entry in fs::read_dir(&repo_dir)? {
            let worktree_entry = worktree_entry?;
            let worktree_dir = worktree_entry.path();
            if !worktree_dir.is_dir() {
                continue;
            }
            let git_marker = worktree_dir.join(".git");
            if !git_marker.exists() {
                issues.push(format!(
                    "{}: does not look like a linked git worktree (no .git)",
                    worktree_dir.display()
                ));
            }
        }
    }
    Ok(())
}

/// Repos with no resolvable org (no `origin` remote, or an unparsable one)
/// are only ever reachable via `find`/`go`'s bare-name substring match; the
/// `org/`, `org/repo` forms and `move`/`get` can never resolve them, since
/// there is no string that means "no org". Flag them so this isn't a silent
/// trap for whoever hits it.
fn check_unattributed_repos(tree: &WorkspaceTree, issues: &mut Vec<String>) {
    for repo in tree.unattributed_repos() {
        issues.push(format!(
            "{}: no resolvable `origin` remote — only reachable via `find`/`go` by bare name, not by org/repo, `move`, or `get`",
            repo.path.display()
        ));
    }
}

fn is_hidden(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_repo(dir: &std::path::Path, remote: Option<&str>) {
        std::fs::create_dir_all(dir).unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["init", "-q"])
            .status()
            .unwrap();
        if let Some(remote) = remote {
            Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(["remote", "add", "origin", remote])
                .status()
                .unwrap();
        }
    }

    #[test]
    fn flags_repo_with_no_origin_remote() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let attributed = root.join("platform").join("commerce").join("with-remote");
        init_repo(&attributed, Some("git@github.com:gymshark/with-remote.git"));

        let orphan = root.join("platform").join("commerce").join("orphan-repo");
        init_repo(&orphan, None);

        let tree = WorkspaceTree::discover_at(root).unwrap();
        let mut issues = Vec::new();
        check_unattributed_repos(&tree, &mut issues);

        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("orphan-repo"));
        assert!(issues[0].contains("find"));
        assert!(issues[0].contains("move"));
    }

    #[test]
    fn does_not_flag_repos_with_a_resolvable_remote() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let repo = root.join("platform").join("commerce").join("clean-repo");
        init_repo(&repo, Some("git@github.com:gymshark/clean-repo.git"));

        let tree = WorkspaceTree::discover_at(root).unwrap();
        let mut issues = Vec::new();
        check_unattributed_repos(&tree, &mut issues);

        assert!(issues.is_empty());
    }

    #[test]
    fn flags_empty_workspace_with_no_agents_md() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let workspace_path = root.join("platform").join("empty-workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tree = WorkspaceTree::discover_at(root).unwrap();
        let mut issues = Vec::new();
        check_workspace(
            &tree,
            "platform",
            "empty-workspace",
            &workspace_path,
            &mut issues,
        )
        .unwrap();

        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("empty workspace"));
    }

    #[test]
    fn does_not_flag_workspace_with_agents_md_even_if_no_repos_yet() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let workspace_path = root.join("platform").join("new-workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();
        std::fs::write(workspace_path.join("AGENTS.md"), "# placeholder").unwrap();

        let tree = WorkspaceTree::discover_at(root).unwrap();
        let mut issues = Vec::new();
        check_workspace(
            &tree,
            "platform",
            "new-workspace",
            &workspace_path,
            &mut issues,
        )
        .unwrap();

        assert!(issues.is_empty());
    }
}
