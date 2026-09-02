//! `wksp init [--path <dir>]`
//!
//! Bootstraps the root of the workspace hierarchy itself: creates the root
//! directory (default `~/Workspaces`, or `--path <dir>`) if missing, and
//! generates its `AGENTS.md`/`README.md` via a headless coding-agent CLI.
//! Creates no domains, workspaces, or repos — see `wksp make` for that.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::interact::input_text;
use crate::scaffold::{
    files_needed_and_description_preamble, generate_files, select_agent, MissingPieces,
};

pub fn run(
    path: Option<&Path>,
    agent_override: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    let root = resolve_root(path)?;

    fs::create_dir_all(&root).with_context(|| format!("failed to create {}", root.display()))?;

    let missing = MissingPieces::for_dir(&root);
    if missing.none_missing() {
        println!(
            "{} already has AGENTS.md and README.md; nothing to generate.",
            root.display()
        );
        return Ok(());
    }

    let description = resolve_description(&missing, description)?;
    let agent = select_agent(agent_override)?;

    let prompt = build_prompt(&root, description.as_deref(), &missing);
    generate_files(&agent, &root, "workspace root", &prompt, &missing)?;

    println!("Ready: {}", root.display());
    Ok(())
}

/// `--path` if given, otherwise `~/Workspaces` (matching
/// `WorkspaceTree::resolve_root`'s default, but this command intentionally
/// does not honor `WKSP_ROOT` — `--path` is the explicit override here,
/// since `init` is about *creating* a root, not discovering an existing one).
fn resolve_root(path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = path {
        return Ok(path.to_path_buf());
    }
    let home = std::env::var("HOME").context("HOME environment variable is not set")?;
    Ok(PathBuf::from(home).join("Workspaces"))
}

fn resolve_description(missing: &MissingPieces, provided: Option<&str>) -> Result<Option<String>> {
    if missing.none_missing() {
        return Ok(None);
    }
    if let Some(provided) = provided {
        return Ok(Some(provided.to_string()));
    }

    let description = input_text(
        "Short description of what this workspace root is for (optional context beyond the standard convention)",
        "Pass --description to supply it non-interactively, or leave it unset to use only the standard convention.",
    )?;
    let trimmed = description.trim();
    Ok(if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    })
}

fn build_prompt(root: &Path, description: Option<&str>, missing: &MissingPieces) -> String {
    let preamble = files_needed_and_description_preamble(
        description,
        missing,
        &root.join("AGENTS.md"),
        &root.join("README.md"),
    );

    format!(
        "You are scaffolding the ROOT documentation for a personal local Git workspace \
hierarchy at `{root}`. This root currently contains no domains, workspaces, or repos yet — it \
is being freshly initialized, so describe the convention itself rather than any specific \
domain's contents.\n\n\
{preamble}\
The hierarchy convention is:\n\n\
    {root}/<domain>/<workspace>/<repo>\n\n\
- domain: a broad, durable business or technical area (e.g. `platform`, `aws`, `personal`). \
Documentation and navigation only — never a Git repository, never an active Kiro invocation \
point.\n\
- workspace: the smallest cohesive unit where repositories share operational context or \
tooling. This is the active Kiro scope and may own a workspace-local `.kiro/`. A substantial \
standalone repository may have its own workspace.\n\
- repo (primary checkout): the one direct checkout of a repository origin, always a direct \
child of its owning workspace — never nested deeper, never a sibling of a domain.\n\
- worktree: a named linked checkout of one primary repository, created only at \
`<domain>/<workspace>/.worktrees/<repo>/<worktree>`, never a substitute for a domain, workspace, \
or primary checkout.\n\n\
AGENTS.md should follow this shape: a `# Workspaces — Agent Guide` (or similarly titled) \
heading; a `## Purpose` section explaining the domain -> workspace -> repo model and that \
domains/workspaces are containers, not Git repositories; a `## Canonical layout` section \
showing the directory tree above; an `## Operating rules` section covering: never `git init` at \
this root or in a domain, primary checkouts are always `<root>/<domain>/<workspace>/<repo>`, Git \
commands should target a known repository root rather than running recursively across a domain \
or workspace, worktrees only ever live under `.worktrees/<repo>/<worktree>` within their owning \
workspace, and being colocated in a workspace never implies shared infrastructure/deployment \
authority. Since this root is freshly initialized with no domains yet, keep the guide focused on \
the convention rather than inventing example domains. README.md should be the human-facing \
equivalent: what this directory is, the same layout, and why the convention exists (shared \
context without repository clutter, clear repository relationships, safe parallel work via \
worktrees).\n\n\
Keep both files concise and accurate to exactly what's described above; don't pad with \
unrelated boilerplate or invent domains/workspaces that don't exist yet.\n",
        root = root.display(),
    )
}
