//! `wksp make <domain>/<workspace>`

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use crate::agent::{detect_installed_agents, run_headless, AgentKind};
use crate::commands::parse_domain_workspace;
use crate::interact::{input_text, select};
use crate::workspace_tree::WorkspaceTree;

/// What's missing at a given level (domain or workspace), driving both the
/// description prompts and the agent instructions.
struct MissingPieces {
    agents_md: bool,
    readme_md: bool,
}

impl MissingPieces {
    fn none_missing(&self) -> bool {
        !self.agents_md && !self.readme_md
    }
}

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

    let domain_missing = missing_pieces(&domain_path);
    let workspace_missing = missing_pieces(&workspace_path);

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
        generate_level(
            agent,
            &domain_path,
            LevelKind::Domain,
            &domain,
            domain_description.as_deref(),
            &domain_missing,
            None,
        )?;
    }

    if !workspace_missing.none_missing() {
        let parent_agents_md = fs::read_to_string(domain_path.join("AGENTS.md")).ok();
        generate_level(
            agent,
            &workspace_path,
            LevelKind::Workspace,
            &workspace,
            workspace_description.as_deref(),
            &workspace_missing,
            parent_agents_md.as_deref(),
        )?;
    }

    println!("Ready: {}", workspace_path.display());
    Ok(())
}

fn missing_pieces(dir: &Path) -> MissingPieces {
    MissingPieces {
        agents_md: !dir.join("AGENTS.md").exists(),
        readme_md: !dir.join("README.md").exists(),
    }
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

fn select_agent(agent_override: Option<&str>) -> Result<AgentKind> {
    if let Some(name) = agent_override {
        return AgentKind::parse(name).ok_or_else(|| {
            anyhow!(
                "unknown agent `{name}` (try: kiro, kiro-cli, opencode, claude, codex, pi, omp)"
            )
        });
    }

    let detected = detect_installed_agents();
    match detected.len() {
        0 => Err(anyhow!(
            "no supported agent CLI was found on PATH (looked for: kiro, kiro-cli, opencode, claude, codex, pi, omp). \
Install one of these, or pass --agent <name> if it's installed under a different PATH entry."
        )),
        1 => Ok(detected[0].kind),
        _ => {
            let labels: Vec<String> = detected.iter().map(|a| a.kind.slug().to_string()).collect();
            let index = select(
                "Multiple agent CLIs detected — which should generate the files?",
                &labels,
                "Pass --agent <name> to choose one non-interactively.",
            )?;
            Ok(detected[index].kind)
        }
    }
}

#[derive(Clone, Copy)]
enum LevelKind {
    Domain,
    Workspace,
}

impl LevelKind {
    fn label(&self) -> &'static str {
        match self {
            LevelKind::Domain => "domain",
            LevelKind::Workspace => "workspace",
        }
    }
}

/// Generate AGENTS.md and/or README.md for one level (domain or workspace).
fn generate_level(
    agent: AgentKind,
    dir: &Path,
    kind: LevelKind,
    name: &str,
    description: Option<&str>,
    missing: &MissingPieces,
    parent_agents_md: Option<&str>,
) -> Result<()> {
    let agents_md_path = dir.join("AGENTS.md");
    let readme_path = dir.join("README.md");

    let prompt = build_prompt(
        kind,
        name,
        description,
        missing,
        &agents_md_path,
        &readme_path,
        parent_agents_md,
    );

    println!(
        "Generating {} files for `{name}` via `{}`...",
        kind.label(),
        agent.slug()
    );
    let output = run_headless(agent, &prompt, dir)?;

    // The agent was instructed to write the files itself. If exactly one file
    // was requested and it still doesn't exist, fall back to writing the
    // agent's raw response there — this never blindly copies one file's
    // content into a differently-purposed file when both were missing.
    let agents_md_missing_after = missing.agents_md && !agents_md_path.exists();
    let readme_missing_after = missing.readme_md && !readme_path.exists();

    match (agents_md_missing_after, readme_missing_after) {
        (true, false) => {
            fs::write(&agents_md_path, &output).with_context(|| {
                format!("fallback write to {} failed", agents_md_path.display())
            })?;
            println!(
                "  note: {} used the agent's raw response as a fallback (the agent didn't write the file itself)",
                agents_md_path.display()
            );
        }
        (false, true) => {
            fs::write(&readme_path, &output)
                .with_context(|| format!("fallback write to {} failed", readme_path.display()))?;
            println!(
                "  note: {} used the agent's raw response as a fallback (the agent didn't write the file itself)",
                readme_path.display()
            );
        }
        (true, true) => {
            println!(
                "  warning: `{}` did not write either requested file. Its response was:\n\n{output}",
                agent.slug()
            );
            anyhow::bail!(
                "{} did not create {} or {}",
                agent.slug(),
                agents_md_path.display(),
                readme_path.display()
            );
        }
        (false, false) => {}
    }

    Ok(())
}

fn build_prompt(
    kind: LevelKind,
    name: &str,
    description: Option<&str>,
    missing: &MissingPieces,
    agents_md_path: &Path,
    readme_path: &Path,
    parent_agents_md: Option<&str>,
) -> String {
    let mut files_needed = Vec::new();
    if missing.agents_md {
        files_needed.push(format!("- `{}`", agents_md_path.display()));
    }
    if missing.readme_md {
        files_needed.push(format!("- `{}`", readme_path.display()));
    }

    let mut prompt = format!(
        "You are scaffolding documentation for a {level} named `{name}` inside a personal \
`domain -> workspace -> repo` local Git workspace hierarchy under `~/Workspaces`.\n\n\
Purpose/description of this {level} (as given by the owner): {description}\n\n\
Create the following file(s) using your own file-write capability (do not just print their \
contents in your response — actually write them to these exact paths):\n{files}\n\n",
        level = kind.label(),
        name = name,
        description = description
            .unwrap_or("(not provided; infer something reasonable and generic from the name)"),
        files = files_needed.join("\n"),
    );

    prompt.push_str(match kind {
        LevelKind::Domain => {
            "AGENTS.md for a domain should follow this shape: a `# <Domain> Domain — Agent Guide` \
heading, a `## Purpose` section, a `## Workspace map` section (a table is fine even if it only \
lists one workspace so far), a `## Domain rules` section stating this is a documentation \
boundary and not a Git repository or active Kiro invocation point, and end with a pointer to \
read `../AGENTS.md` first and see `./README.md` for the human-facing view. README.md should be \
the human-facing equivalent: what this domain is for and what lives in it.\n"
        }
        LevelKind::Workspace => {
            "AGENTS.md for a workspace should follow this shape: a `# <Workspace> Workspace — Agent \
Guide` heading, a `## Purpose` section, a `## Repositories` section (bullet list; it may be empty \
or aspirational if no repos are checked out yet), a `## Working rules` section (start Kiro here \
for cross-repo work, start in a repo for repo-scoped work, keep worktrees at \
`.worktrees/<repo>/<worktree>`), and end with a pointer to read the parent `../AGENTS.md` and root \
guidance before working. README.md should be the human-facing equivalent.\n"
        }
    });

    if let Some(parent) = parent_agents_md {
        prompt.push_str(&format!(
            "\nFor tone and formatting consistency, here is the parent AGENTS.md this new file sits \
under:\n\n---\n{parent}\n---\n"
        ));
    }

    prompt.push_str("\nKeep both files concise and specific to the description given; don't pad with generic boilerplate.\n");

    prompt
}
