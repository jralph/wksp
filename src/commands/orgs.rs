//! `wksp orgs`

use anyhow::Result;

use crate::workspace_tree::WorkspaceTree;

pub fn run() -> Result<()> {
    let tree = WorkspaceTree::discover()?;

    if tree.repos.is_empty() {
        println!("No repos indexed under {}.", tree.root.display());
        return Ok(());
    }

    for org in tree.orgs() {
        let count = tree.find_by_org(&org).len();
        let noun = if count == 1 { "repo" } else { "repos" };
        println!("{org:<30} {count} {noun}");
    }

    let unattributed = tree.repos.iter().filter(|r| r.org.is_none()).count();
    if unattributed > 0 {
        println!("(no origin remote / unparsable): {unattributed} repo(s)");
    }

    Ok(())
}
