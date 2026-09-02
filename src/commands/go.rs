//! `wksp go <repo>`
//!
//! Contract: on success, prints *only* the resolved workspace path to
//! stdout, with no trailing decoration, so `wksp shell <shell>`'s wrapper
//! function can capture it and `cd`. Everything else (prompts, errors,
//! disambiguation UI) goes to stderr.

use anyhow::{bail, Result};

use crate::interact::select;
use crate::workspace_tree::{Repo, WorkspaceTree};

pub fn run(query: &str) -> Result<()> {
    let tree = WorkspaceTree::discover()?;
    let matches = tree.resolve_query(query);

    let chosen: &Repo = match matches.len() {
        0 => bail!("no repos matched `{query}`"),
        1 => matches[0],
        _ => pick_one(&matches)?,
    };

    println!("{}", chosen.workspace_path().display());
    Ok(())
}

fn pick_one<'a>(matches: &[&'a Repo]) -> Result<&'a Repo> {
    let labels: Vec<String> = matches
        .iter()
        .map(|r| format!("{}  ({})", r.org_slug(), r.location()))
        .collect();

    let index = select(
        "Multiple repos matched. Which one did you mean?",
        &labels,
        "Use a more specific query, e.g. the exact `org/repo` form.",
    )?;

    Ok(matches[index])
}
