//! Shared agent-invocation and file-generation logic for `wksp make` and
//! `wksp init`. Both commands need the same thing: given a directory that
//! may be missing `AGENTS.md`/`README.md`, pick an agent CLI, build a
//! level-appropriate prompt, invoke it headlessly, and fall back to writing
//! its raw response if it didn't create the file(s) itself.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::agent::{detect_installed_agents, run_headless, AgentKind};
use crate::interact::select;

/// What's missing in a given directory, driving both the description prompt
/// and the agent instructions.
pub struct MissingPieces {
    pub agents_md: bool,
    pub readme_md: bool,
}

impl MissingPieces {
    pub fn for_dir(dir: &Path) -> Self {
        Self {
            agents_md: !dir.join("AGENTS.md").exists(),
            readme_md: !dir.join("README.md").exists(),
        }
    }

    pub fn none_missing(&self) -> bool {
        !self.agents_md && !self.readme_md
    }
}

/// The agent CLI chosen to generate files, plus the exact binary to invoke.
pub struct SelectedAgent {
    pub kind: AgentKind,
    pub binary: PathBuf,
}

/// Resolve which agent to use: an explicit `--agent` override, the single
/// agent detected on PATH, an interactive pick among several, or a clear
/// error if none are installed.
pub fn select_agent(agent_override: Option<&str>) -> Result<SelectedAgent> {
    if let Some(name) = agent_override {
        let kind = AgentKind::parse(name).ok_or_else(|| {
            anyhow!(
                "unknown agent `{name}` (try: kiro, kiro-cli, opencode, claude, codex, pi, omp)"
            )
        })?;
        // No detection was done for an explicit override; fall back to a
        // bare binary name and let the OS resolve it via PATH at exec time.
        return Ok(SelectedAgent {
            kind,
            binary: PathBuf::from(kind.binary_name()),
        });
    }

    let detected = detect_installed_agents();
    match detected.len() {
        0 => Err(anyhow!(
            "no supported agent CLI was found on PATH (looked for: kiro, kiro-cli, opencode, claude, codex, pi, omp). \
Install one of these, or pass --agent <name> if it's installed under a different PATH entry."
        )),
        1 => Ok(SelectedAgent {
            kind: detected[0].kind,
            binary: detected[0].binary_path.clone(),
        }),
        _ => {
            let labels: Vec<String> = detected.iter().map(|a| a.kind.slug().to_string()).collect();
            let index = select(
                "Multiple agent CLIs detected — which should generate the files?",
                &labels,
                "Pass --agent <name> to choose one non-interactively.",
            )?;
            Ok(SelectedAgent {
                kind: detected[index].kind,
                binary: detected[index].binary_path.clone(),
            })
        }
    }
}

/// Invoke `agent` headlessly in `dir` with `prompt`, then ensure the file(s)
/// described by `missing` exist afterward — falling back to writing the
/// agent's raw response when exactly one file is still missing, and erroring
/// out (never silently doing nothing) when both still are.
pub fn generate_files(
    agent: &SelectedAgent,
    dir: &Path,
    label: &str,
    prompt: &str,
    missing: &MissingPieces,
) -> Result<()> {
    let agents_md_path = dir.join("AGENTS.md");
    let readme_path = dir.join("README.md");

    println!("Generating {label} files via `{}`...", agent.kind.slug());
    let output = run_headless(agent.kind, &agent.binary, prompt, dir)?;

    // The agent was instructed to write the files itself. If exactly one
    // file was requested and it still doesn't exist, fall back to writing
    // the agent's raw response there — this never blindly copies one file's
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
                agent.kind.slug()
            );
            anyhow::bail!(
                "{} did not create {} or {}",
                agent.kind.slug(),
                agents_md_path.display(),
                readme_path.display()
            );
        }
        (false, false) => {}
    }

    Ok(())
}

/// Build the shared framing every prompt starts with: which files are
/// needed, and the owner-supplied (or absent) description.
pub fn files_needed_and_description_preamble(
    description: Option<&str>,
    missing: &MissingPieces,
    agents_md_path: &Path,
    readme_path: &Path,
) -> String {
    let mut files_needed = Vec::new();
    if missing.agents_md {
        files_needed.push(format!("- `{}`", agents_md_path.display()));
    }
    if missing.readme_md {
        files_needed.push(format!("- `{}`", readme_path.display()));
    }

    format!(
        "Purpose/description (as given by the owner): {description}\n\n\
Create the following file(s) using your own file-write capability (do not just print their \
contents in your response — actually write them to these exact paths):\n{files}\n\n",
        description = description
            .unwrap_or("(not provided; infer something reasonable and generic from the name)"),
        files = files_needed.join("\n"),
    )
}
