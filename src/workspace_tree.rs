//! Discovery and indexing of the `~/Workspaces` domain -> workspace -> repo hierarchy.
//!
//! The tree root defaults to `~/Workspaces` but can be overridden with the
//! `WKSP_ROOT` environment variable (primarily used by tests and by anyone
//! running a parallel hierarchy).
//!
//! A repository's "org" is never inferred from its directory name. It is
//! parsed from the `origin` remote URL, matching the convention documented in
//! the root `AGENTS.md` (e.g. `gymshark/repo-name`, `jralph/repo-name`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::Serialize;

/// Directory name reserved for linked Git worktrees within a workspace.
pub const WORKTREES_DIR: &str = ".worktrees";

/// A single discovered repository checkout.
#[derive(Debug, Clone, Serialize)]
pub struct Repo {
    /// Repository directory name, e.g. `customer-service`.
    pub name: String,
    /// Org/owner parsed from the `origin` remote, e.g. `gymshark`. `None` if
    /// the repo has no `origin` remote or it could not be parsed.
    pub org: Option<String>,
    /// Domain name, e.g. `platform`.
    pub domain: String,
    /// Workspace name, e.g. `customer-identity`.
    pub workspace: String,
    /// Absolute path to the repository checkout.
    pub path: PathBuf,
}

impl Repo {
    /// The workspace directory containing this repo (its parent directory).
    pub fn workspace_path(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.path.clone())
    }

    /// `org/name` if an org was resolved, otherwise just `name`.
    pub fn org_slug(&self) -> String {
        match &self.org {
            Some(org) => format!("{org}/{name}", org = org, name = self.name),
            None => self.name.clone(),
        }
    }

    /// `domain/workspace` for this repo.
    pub fn location(&self) -> String {
        format!("{}/{}", self.domain, self.workspace)
    }
}

/// The full indexed view of the workspace tree.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceTree {
    pub root: PathBuf,
    pub repos: Vec<Repo>,
}

impl WorkspaceTree {
    /// Discover the workspace root, defaulting to `~/Workspaces` unless
    /// `WKSP_ROOT` is set.
    pub fn resolve_root() -> Result<PathBuf> {
        if let Ok(root) = std::env::var("WKSP_ROOT") {
            return Ok(PathBuf::from(root));
        }
        let home = std::env::var("HOME").context("HOME environment variable is not set")?;
        Ok(PathBuf::from(home).join("Workspaces"))
    }

    /// Walk the tree rooted at `resolve_root()` and build a full index.
    ///
    /// This performs a fresh filesystem walk plus one `git remote get-url
    /// origin` subprocess per discovered repo. For the scale this tool is
    /// designed for (hundreds of repos) that is fast enough to not warrant
    /// caching; if that changes, caching should live here behind the same
    /// public API.
    pub fn discover() -> Result<Self> {
        let root = Self::resolve_root()?;
        Self::discover_at(root)
    }

    /// Same as [`discover`], but against an explicit root. Exposed for
    /// testing and for commands that already resolved the root once.
    pub fn discover_at(root: PathBuf) -> Result<Self> {
        let mut repos = Vec::new();

        if !root.exists() {
            return Ok(Self { root, repos });
        }

        for domain_entry in read_dirs_sorted(&root)? {
            let domain_name = match dir_name(&domain_entry) {
                Some(n) => n,
                None => continue,
            };

            for workspace_entry in read_dirs_sorted(&domain_entry)? {
                let workspace_name = match dir_name(&workspace_entry) {
                    Some(n) => n,
                    None => continue,
                };

                for repo_entry in read_dirs_sorted(&workspace_entry)? {
                    let repo_name = match dir_name(&repo_entry) {
                        Some(n) => n,
                        None => continue,
                    };

                    // .worktrees holds linked worktrees, never primary repos.
                    if repo_name == WORKTREES_DIR {
                        continue;
                    }

                    if !is_git_repo(&repo_entry) {
                        continue;
                    }

                    let org = detect_origin_org(&repo_entry);

                    repos.push(Repo {
                        name: repo_name,
                        org,
                        domain: domain_name.clone(),
                        workspace: workspace_name.clone(),
                        path: repo_entry,
                    });
                }
            }
        }

        repos.sort_by(|a, b| a.location().cmp(&b.location()).then(a.name.cmp(&b.name)));

        Ok(Self { root, repos })
    }

    /// All distinct domain names, sorted.
    pub fn domains(&self) -> Vec<String> {
        let mut domains: Vec<String> = self.repos.iter().map(|r| r.domain.clone()).collect();
        domains.sort();
        domains.dedup();
        domains
    }

    /// All distinct orgs found across indexed repos, sorted.
    pub fn orgs(&self) -> Vec<String> {
        let mut orgs: Vec<String> = self.repos.iter().filter_map(|r| r.org.clone()).collect();
        orgs.sort();
        orgs.dedup();
        orgs
    }

    /// Exact lookup by `org/repo`.
    pub fn find_exact(&self, org: &str, repo: &str) -> Option<&Repo> {
        self.repos
            .iter()
            .find(|r| r.org.as_deref() == Some(org) && r.name == repo)
    }

    /// All repos under a given org (case-sensitive exact org match).
    pub fn find_by_org(&self, org: &str) -> Vec<&Repo> {
        self.repos
            .iter()
            .filter(|r| r.org.as_deref() == Some(org))
            .collect()
    }

    /// Substring match against repo name, case-insensitive.
    pub fn find_by_name(&self, name: &str) -> Vec<&Repo> {
        let needle = name.to_lowercase();
        self.repos
            .iter()
            .filter(|r| r.name.to_lowercase().contains(&needle))
            .collect()
    }

    /// Whether `org` is a known org in this index.
    pub fn is_known_org(&self, org: &str) -> bool {
        self.repos.iter().any(|r| r.org.as_deref() == Some(org))
    }

    /// All repos within a domain, and optionally a specific workspace.
    pub fn find_by_location(&self, domain: &str, workspace: Option<&str>) -> Vec<&Repo> {
        self.repos
            .iter()
            .filter(|r| r.domain == domain && workspace.map(|w| r.workspace == w).unwrap_or(true))
            .collect()
    }

    /// Resolve a `find`/`go` style query string against this index.
    ///
    /// The query has three forms, disambiguated by shape rather than by
    /// guessing whether a token is a known org:
    ///
    ///   - `org/`      trailing slash explicitly requests every repo under `org`.
    ///   - `org/repo`  an exact lookup for one specific repo.
    ///   - `repo`      (no slash) a case-insensitive substring match on repo name.
    pub fn resolve_query(&self, query: &str) -> Vec<&Repo> {
        if let Some(org) = query.strip_suffix('/') {
            return self.find_by_org(org);
        }

        if let Some((org, repo)) = query.split_once('/') {
            return self.find_exact(org, repo).into_iter().collect::<Vec<_>>();
        }

        self.find_by_name(query)
    }
}

fn read_dirs_sorted(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect(),
        Err(_) => return Ok(Vec::new()),
    };
    entries.sort();
    Ok(entries)
}

fn dir_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.starts_with('.'))
        .map(str::to_string)
}

fn is_git_repo(path: &Path) -> bool {
    let git_path = path.join(".git");
    git_path.exists()
}

/// Resolve the `origin` remote's org via `git remote get-url origin`.
/// Returns `None` on any failure (no remote, not a repo, git not on PATH).
fn detect_origin_org(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8(output.stdout).ok()?;
    parse_org_from_remote(url.trim())
}

/// Parse the org/owner segment out of a remote URL. Supports:
/// - `git@host:org/repo.git`
/// - `https://host/org/repo.git` / `https://host/org/repo`
/// - `ssh://git@host/org/repo.git`
pub fn parse_org_from_remote(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let path_part = if let Some(rest) = url.strip_prefix("git@") {
        // git@host:org/repo.git
        rest.split_once(':').map(|(_, p)| p)
    } else if let Some(rest) = url
        .strip_prefix("ssh://")
        .or_else(|| url.strip_prefix("https://"))
        .or_else(|| url.strip_prefix("http://"))
    {
        // host/org/repo(.git)
        rest.split_once('/').map(|(_, p)| p)
    } else {
        None
    }?;

    let mut segments: Vec<&str> = path_part.trim_matches('/').split('/').collect();
    // ssh:// URLs may carry a port-prefixed host segment already stripped;
    // guard against empty leading segments from double slashes.
    segments.retain(|s| !s.is_empty());

    if segments.len() < 2 {
        return None;
    }

    let org = segments[segments.len() - 2];
    if org.is_empty() {
        None
    } else {
        Some(org.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_repo_with_remote(dir: &Path, remote: &str) {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["init", "-q"])
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["remote", "add", "origin", remote])
            .status()
            .unwrap();
    }

    #[test]
    fn parses_ssh_style_remote() {
        assert_eq!(
            parse_org_from_remote("git@github.com:gymshark/aws-oidc.git"),
            Some("gymshark".to_string())
        );
    }

    #[test]
    fn parses_https_style_remote() {
        assert_eq!(
            parse_org_from_remote("https://github.com/jralph/wksp.git"),
            Some("jralph".to_string())
        );
        assert_eq!(
            parse_org_from_remote("https://github.com/jralph/wksp"),
            Some("jralph".to_string())
        );
    }

    #[test]
    fn parses_ssh_scheme_remote() {
        assert_eq!(
            parse_org_from_remote("ssh://git@github.com/jralph/wksp.git"),
            Some("jralph".to_string())
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_org_from_remote(""), None);
        assert_eq!(parse_org_from_remote("not-a-url"), None);
    }

    #[test]
    fn discovers_domain_workspace_repo_tree() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let repo_path = root
            .join("platform")
            .join("customer-identity")
            .join("customer-service");
        fs::create_dir_all(&repo_path).unwrap();
        init_repo_with_remote(&repo_path, "git@github.com:gymshark/customer-service.git");

        // A non-git directory should be ignored.
        let non_repo = root
            .join("platform")
            .join("customer-identity")
            .join("notes");
        fs::create_dir_all(&non_repo).unwrap();

        // .worktrees must never be treated as a repo even if it looks like one.
        let worktrees = root
            .join("platform")
            .join("customer-identity")
            .join(WORKTREES_DIR);
        fs::create_dir_all(&worktrees).unwrap();

        let tree = WorkspaceTree::discover_at(root.clone()).unwrap();

        assert_eq!(tree.repos.len(), 1);
        let repo = &tree.repos[0];
        assert_eq!(repo.name, "customer-service");
        assert_eq!(repo.org, Some("gymshark".to_string()));
        assert_eq!(repo.domain, "platform");
        assert_eq!(repo.workspace, "customer-identity");
        assert_eq!(repo.path, repo_path);
        assert_eq!(
            repo.workspace_path(),
            root.join("platform").join("customer-identity")
        );
        assert_eq!(repo.org_slug(), "gymshark/customer-service");
        assert_eq!(repo.location(), "platform/customer-identity");
    }

    #[test]
    fn find_helpers_behave_as_expected() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let repo_path = root.join("aws").join("isb").join("isb-management");
        fs::create_dir_all(&repo_path).unwrap();
        init_repo_with_remote(&repo_path, "git@github.com:gymshark/isb-management.git");

        let personal_repo = root
            .join("personal")
            .join("git-manager")
            .join("git-manager-go");
        fs::create_dir_all(&personal_repo).unwrap();
        init_repo_with_remote(&personal_repo, "git@github.com:jralph/git-manager-go.git");

        let tree = WorkspaceTree::discover_at(root).unwrap();

        assert!(tree.is_known_org("gymshark"));
        assert!(tree.is_known_org("jralph"));
        assert!(!tree.is_known_org("no-such-org"));

        assert_eq!(tree.find_by_org("gymshark").len(), 1);
        assert_eq!(tree.find_by_name("git-manager").len(), 1);
        assert_eq!(
            tree.find_exact("jralph", "git-manager-go").unwrap().name,
            "git-manager-go"
        );
        assert!(tree.find_exact("jralph", "does-not-exist").is_none());

        assert_eq!(tree.find_by_location("aws", None).len(), 1);
        assert_eq!(tree.find_by_location("aws", Some("isb")).len(), 1);
        assert_eq!(
            tree.find_by_location("aws", Some("no-such-workspace"))
                .len(),
            0
        );

        let mut domains = tree.domains();
        domains.sort();
        assert_eq!(domains, vec!["aws".to_string(), "personal".to_string()]);
    }

    #[test]
    fn resolve_query_disambiguates_by_shape() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let repo_a = root.join("aws").join("isb").join("isb-management");
        fs::create_dir_all(&repo_a).unwrap();
        init_repo_with_remote(&repo_a, "git@github.com:gymshark/isb-management.git");

        let repo_b = root.join("aws").join("isb").join("aws-go-isb-client");
        fs::create_dir_all(&repo_b).unwrap();
        init_repo_with_remote(&repo_b, "git@github.com:gymshark/aws-go-isb-client.git");

        let repo_c = root
            .join("personal")
            .join("git-manager")
            .join("git-manager-go");
        fs::create_dir_all(&repo_c).unwrap();
        init_repo_with_remote(&repo_c, "git@github.com:jralph/git-manager-go.git");

        let tree = WorkspaceTree::discover_at(root).unwrap();

        // Trailing slash -> every repo under that org.
        let org_results = tree.resolve_query("gymshark/");
        assert_eq!(org_results.len(), 2);

        // org/repo -> exact single lookup.
        let exact_results = tree.resolve_query("jralph/git-manager-go");
        assert_eq!(exact_results.len(), 1);
        assert_eq!(exact_results[0].name, "git-manager-go");

        // org/repo that doesn't exist -> no results, not an org listing.
        assert_eq!(tree.resolve_query("jralph/does-not-exist").len(), 0);

        // bare token -> substring match on repo name, across all orgs.
        let name_results = tree.resolve_query("isb");
        assert_eq!(name_results.len(), 2);
    }
}
