//! `wksp make <domain>/<workspace>`

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::commands::parse_domain_workspace;
use crate::interact::input_text;
use crate::scaffold::{
    files_needed_and_description_preamble, generate_files, select_agent, MissingPieces,
};
use crate::workspace_tree::WorkspaceTree;

pub fn run(
    domain_workspace: &str,
    agent_override: Option<&str>,
    domain_description: Option<&str>,
    workspace_description: Option<&str>,
    // Accepted for signature symmetry with the yes/no-confirmation commands
    // that may invoke `make` on our behalf (see `ensure_destination_ready`);
    // `make` itself has no yes/no confirmation of its own.
    _assume_yes: bool,
) -> Result<()> {
    let (domain, workspace) = parse_domain_workspace(domain_workspace)?;
    let tree = WorkspaceTree::discover()?;

    let domain_path = tree.root.join(&domain);
    let workspace_path = domain_path.join(&workspace);

    fs::create_dir_all(&workspace_path)
        .with_context(|| format!("failed to create {}", workspace_path.display()))?;

    let domain_missing = MissingPieces::for_dir(&domain_path);
    let workspace_missing = MissingPieces::for_dir(&workspace_path);

    if domain_missing.none_missing() && workspace_missing.none_missing() {
        println!(
            "{domain}/{workspace} already has AGENTS.md and README.md at both levels; nothing to generate."
        );
        return Ok(());
    }

    let domain_description = resolve_description(
        &domain_missing,
        domain_description,
        &format!("Short description of the `{domain}` domain's purpose"),
    )?;
    let workspace_description = resolve_description(
        &workspace_missing,
        workspace_description,
        &format!("Short description of the `{workspace}` workspace's purpose"),
    )?;

    let agent = select_agent(agent_override)?;

    if !domain_missing.none_missing() {
        let prompt = build_domain_prompt(
            &domain,
            domain_description.as_deref(),
            &domain_missing,
            &domain_path,
        );
        generate_files(&agent, &domain_path, "domain", &prompt, &domain_missing)?;
    }

    if !workspace_missing.none_missing() {
        let parent_agents_md = fs::read_to_string(domain_path.join("AGENTS.md")).ok();
        let prompt = build_workspace_prompt(
            &workspace,
            workspace_description.as_deref(),
            &workspace_missing,
            &workspace_path,
            parent_agents_md.as_deref(),
        );
        generate_files(
            &agent,
            &workspace_path,
            "workspace",
            &prompt,
            &workspace_missing,
        )?;
    }

    println!("Ready: {}", workspace_path.display());
    Ok(())
}

fn resolve_description(
    missing: &MissingPieces,
    provided: Option<&str>,
    prompt: &str,
) -> Result<Option<String>> {
    if missing.none_missing() {
        return Ok(None);
    }
    if let Some(provided) = provided {
        return Ok(Some(provided.to_string()));
    }

    let description = input_text(
        prompt,
        "Pass --domain-description and/or --workspace-description to supply it non-interactively.",
    )?;
    Ok(Some(description))
}

fn build_domain_prompt(
    name: &str,
    description: Option<&str>,
    missing: &MissingPieces,
    dir: &Path,
) -> String {
    let preamble = files_needed_and_description_preamble(
        description,
        missing,
        &dir.join("AGENTS.md"),
        &dir.join("README.md"),
    );

    format!(
        "You are scaffolding documentation for a domain named `{name}` inside a personal \
`domain -> workspace -> repo` local Git workspace hierarchy under `~/Workspaces`.\n\n\
{preamble}\
AGENTS.md for a domain should follow this shape: a `# {name} Domain — Agent Guide` heading, a \
`## Purpose` section, a `## Workspace map` section (a table is fine even if it only lists one \
workspace so far), a `## Domain rules` section stating this is a documentation boundary and not \
a Git repository or active Kiro invocation point, and end with a pointer to read `../AGENTS.md` \
first and see `./README.md` for the human-facing view. README.md should be the human-facing \
equivalent: what this domain is for and what lives in it.\n\n\
Keep both files concise and specific to the description given; don't pad with generic boilerplate.\n"
    )
}

fn build_workspace_prompt(
    name: &str,
    description: Option<&str>,
    missing: &MissingPieces,
    dir: &Path,
    parent_agents_md: Option<&str>,
) -> String {
    let preamble = files_needed_and_description_preamble(
        description,
        missing,
        &dir.join("AGENTS.md"),
        &dir.join("README.md"),
    );

    let mut prompt = format!(
        "You are scaffolding documentation for a workspace named `{name}` inside a personal \
`domain -> workspace -> repo` local Git workspace hierarchy under `~/Workspaces`.\n\n\
{preamble}\
AGENTS.md for a workspace should follow this shape: a `# {name} Workspace — Agent Guide` \
heading, a `## Purpose` section, a `## Repositories` section (bullet list; it may be empty or \
aspirational if no repos are checked out yet), a `## Working rules` section (start Kiro here \
for cross-repo work, start in a repo for repo-scoped work, keep worktrees at \
`.worktrees/<repo>/<worktree>`), and end with a pointer to read the parent `../AGENTS.md` and \
root guidance before working. README.md should be the human-facing equivalent.\n"
    );

    if let Some(parent) = parent_agents_md {
        prompt.push_str(&format!(
            "\nFor tone and formatting consistency, here is the parent AGENTS.md this new file sits \
under:\n\n---\n{parent}\n---\n"
        ));
    }

    prompt.push_str("\nKeep both files concise and specific to the description given; don't pad with generic boilerplate.\n");

    prompt
}
