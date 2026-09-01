//! `wksp move <org>/<repo> <domain>/<workspace>`

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::commands::{
    ensure_destination_ready, parse_domain_workspace, parse_org_repo, repo_target_path,
};
use crate::workspace_tree::WorkspaceTree;

pub fn run(org_repo: &str, destination: &str, assume_yes: bool) -> Result<()> {
    let (org, repo_name) = parse_org_repo(org_repo)?;
    let (domain, workspace) = parse_domain_workspace(destination)?;

    let tree = WorkspaceTree::discover()?;

    let source = tree
        .find_exact(&org, &repo_name)
        .with_context(|| {
            format!(
                "`{org_repo}` is not checked out anywhere under {}",
                tree.root.display()
            )
        })?
        .path
        .clone();

    let dest_path = repo_target_path(&tree.root, &domain, &workspace, &repo_name);

    if dest_path == source {
        println!("`{org_repo}` is already at {domain}/{workspace}.");
        return Ok(());
    }

    if dest_path.exists() {
        bail!(
            "a repo already exists at {} — refusing to overwrite it",
            dest_path.display()
        );
    }

    if !ensure_destination_ready(&tree, &domain, &workspace, assume_yes)? {
        println!("Move cancelled.");
        return Ok(());
    }

    move_directory(&source, &dest_path)?;

    println!(
        "Moved {org_repo}: {} -> {}",
        source.display(),
        dest_path.display()
    );
    Ok(())
}

/// Move `source` to `dest`, falling back to copy+remove if they're on
/// different filesystems (rename(2) cannot cross volumes).
fn move_directory(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }

    match fs::rename(source, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir_recursive(source, dest).with_context(|| {
                format!("failed to copy {} to {}", source.display(), dest.display())
            })?;
            fs::remove_dir_all(source).with_context(|| {
                format!(
                    "copied to {} but failed to remove original {}",
                    dest.display(),
                    source.display()
                )
            })?;
            Ok(())
        }
    }
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
