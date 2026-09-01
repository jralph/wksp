//! `wksp find <repo>`

use anyhow::Result;

use crate::workspace_tree::{Repo, WorkspaceTree};

pub fn run(query: &str) -> Result<()> {
    let tree = WorkspaceTree::discover()?;
    let matches = tree.resolve_query(query);

    if matches.is_empty() {
        println!("No repos matched `{query}`.");
        return Ok(());
    }

    print_matches(&matches);
    Ok(())
}

/// Print matches grouped by domain/workspace. Used by `find`; `go`'s
/// multi-match picker renders its own single-line-per-match labels instead,
/// since a picker needs one selectable label per match rather than a
/// location-grouped listing.
pub fn print_matches(matches: &[&Repo]) {
    let mut last_location: Option<String> = None;
    for repo in matches {
        let location = repo.location();
        if last_location.as_deref() != Some(location.as_str()) {
            println!("{location}");
            last_location = Some(location);
        }
        println!("  {:<40} {}", repo.org_slug(), repo.path.display());
    }
}
