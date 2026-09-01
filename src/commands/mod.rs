//! Subcommand implementations. Each submodule owns one `wksp` subcommand.

pub mod doctor;
pub mod find;
pub mod get;
pub mod go;
pub mod init;
pub mod list;
pub mod make;
pub mod move_cmd;
pub mod orgs;
pub mod status;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::interact::confirm;
use crate::workspace_tree::WorkspaceTree;

/// Parse a `domain/workspace` argument into its two components, rejecting
/// anything that isn't exactly two non-empty segments.
pub fn parse_domain_workspace(input: &str) -> Result<(String, String)> {
    let mut parts = input.splitn(2, '/');
    let domain = parts.next().unwrap_or("").trim();
    let workspace = parts.next().unwrap_or("").trim();

    if domain.is_empty() || workspace.is_empty() {
        anyhow::bail!("expected `<domain>/<workspace>`, got `{input}`");
    }

    Ok((domain.to_string(), workspace.to_string()))
}

/// Parse an `org/repo` argument into its two components.
pub fn parse_org_repo(input: &str) -> Result<(String, String)> {
    let mut parts = input.splitn(2, '/');
    let org = parts.next().unwrap_or("").trim();
    let repo = parts.next().unwrap_or("").trim();

    if org.is_empty() || repo.is_empty() {
        anyhow::bail!("expected `<org>/<repo>`, got `{input}`");
    }

    Ok((org.to_string(), repo.to_string()))
}

/// Whether `domain`/`workspace` already exist as directories under `root`.
pub fn destination_exists(root: &Path, domain: &str, workspace: &str) -> bool {
    root.join(domain).join(workspace).is_dir()
}

/// Ensure `domain/workspace` exists under `tree.root`, prompting to run
/// `wksp make` if it doesn't. Used by `move` and `get` so a destination is
/// never silently created as a bare, undocumented directory.
///
/// Returns `Ok(true)` if the destination is ready to receive a repo (already
/// existed, or was just created via `make`), `Ok(false)` if the user declined
/// and the caller should abort its own operation without error.
pub fn ensure_destination_ready(
    tree: &WorkspaceTree,
    domain: &str,
    workspace: &str,
    assume_yes: bool,
) -> Result<bool> {
    if destination_exists(&tree.root, domain, workspace) {
        return Ok(true);
    }

    println!(
        "`{domain}/{workspace}` does not exist yet under {}.",
        tree.root.display()
    );
    let proceed = confirm(
        &format!("Create it now by running `wksp make {domain}/{workspace}`?"),
        assume_yes,
        true,
    )?;

    if !proceed {
        return Ok(false);
    }

    crate::commands::make::run(
        &format!("{domain}/{workspace}"),
        None,
        None,
        None,
        assume_yes,
    )?;
    Ok(true)
}

/// The absolute path a repo would live at once created under `domain/workspace`.
pub fn repo_target_path(root: &Path, domain: &str, workspace: &str, repo_name: &str) -> PathBuf {
    root.join(domain).join(workspace).join(repo_name)
}
