//! `wksp get <org>/<repo> <domain>/<workspace>`

use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::commands::{
    ensure_destination_ready, parse_domain_workspace, parse_org_repo, repo_target_path,
};
use crate::interact::confirm;
use crate::workspace_tree::WorkspaceTree;

pub fn run(org_repo: &str, destination: &str, assume_yes: bool) -> Result<()> {
    let (org, repo_name) = parse_org_repo(org_repo)?;
    let (domain, workspace) = parse_domain_workspace(destination)?;

    let tree = WorkspaceTree::discover()?;
    let dest_path = repo_target_path(&tree.root, &domain, &workspace, &repo_name);

    if let Some(existing) = tree.find_exact(&org, &repo_name) {
        if existing.path == dest_path {
            println!(
                "`{org_repo}` is already checked out at {}.",
                dest_path.display()
            );
            return Ok(());
        }

        println!(
            "`{org_repo}` is already checked out at {} ({}).",
            existing.path.display(),
            existing.location()
        );
        let move_it = confirm(
            &format!("Move it to {domain}/{workspace} instead of cloning a duplicate?"),
            assume_yes,
            true,
        )?;

        if move_it {
            crate::commands::move_cmd::run(org_repo, destination, assume_yes)?;
        } else {
            println!("Leaving the existing checkout in place; nothing cloned.");
        }
        return Ok(());
    }

    if dest_path.exists() {
        bail!(
            "{} already exists but is not recognised as `{org_repo}` — refusing to clone over it",
            dest_path.display()
        );
    }

    if !ensure_destination_ready(&tree, &domain, &workspace, assume_yes)? {
        println!("Clone cancelled.");
        return Ok(());
    }

    clone_repo(&org, &repo_name, &dest_path)?;
    println!("Cloned {org_repo} into {}.", dest_path.display());
    Ok(())
}

fn clone_repo(org: &str, repo: &str, dest_path: &std::path::Path) -> Result<()> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }

    let clone_url = format!("git@github.com:{org}/{repo}.git");

    let status = Command::new("git")
        .arg("clone")
        .arg(&clone_url)
        .arg(dest_path)
        .status()
        .with_context(|| format!("failed to execute `git clone {clone_url}`"))?;

    if !status.success() {
        bail!("`git clone {clone_url}` failed with {status}");
    }

    Ok(())
}
