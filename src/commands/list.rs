//! `wksp list [domain]`

use std::collections::BTreeMap;

use anyhow::Result;

use crate::workspace_tree::WorkspaceTree;

pub fn run(domain: Option<&str>) -> Result<()> {
    let tree = WorkspaceTree::discover()?;

    match domain {
        Some(domain) => list_domain(&tree, domain),
        None => list_all_domains(&tree),
    }

    Ok(())
}

fn list_all_domains(tree: &WorkspaceTree) {
    if tree.repos.is_empty() {
        println!("No repos indexed under {}.", tree.root.display());
        return;
    }

    // domain -> workspace -> repo count
    let mut by_domain: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for repo in &tree.repos {
        *by_domain
            .entry(&repo.domain)
            .or_default()
            .entry(&repo.workspace)
            .or_insert(0) += 1;
    }

    for (domain, workspaces) in &by_domain {
        println!("{domain}");
        for (workspace, count) in workspaces {
            let noun = if *count == 1 { "repo" } else { "repos" };
            println!("  {workspace:<30} {count} {noun}");
        }
    }
}

fn list_domain(tree: &WorkspaceTree, domain: &str) {
    let matches = tree.find_by_location(domain, None);
    if matches.is_empty() {
        println!("No repos found under domain `{domain}`.");
        return;
    }

    let mut by_workspace: BTreeMap<&str, Vec<&crate::workspace_tree::Repo>> = BTreeMap::new();
    for repo in matches {
        by_workspace.entry(&repo.workspace).or_default().push(repo);
    }

    for (workspace, mut repos) in by_workspace {
        repos.sort_by(|a, b| a.name.cmp(&b.name));
        println!("{domain}/{workspace}");
        for repo in repos {
            println!("  {:<40} {}", repo.org_slug(), repo.path.display());
        }
    }
}
