//! Detection and headless invocation of coding-agent CLIs, used by `wksp make`
//! to generate `AGENTS.md` / `README.md` content.
//!
//! Supported out of the box: kiro, kiro-cli, opencode, claude (Claude Code),
//! codex, pi, omp. Detection is a PATH lookup only; no agent is invoked
//! unless explicitly selected via `--agent` or the interactive picker.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

/// A coding-agent CLI this tool knows how to drive headlessly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Kiro,
    KiroCli,
    OpenCode,
    Pi,
    Omp,
    ClaudeCode,
    Codex,
}

impl AgentKind {
    /// All agents supported out of the box, in the order they should be
    /// offered to the user.
    pub const ALL: [AgentKind; 7] = [
        AgentKind::Kiro,
        AgentKind::KiroCli,
        AgentKind::OpenCode,
        AgentKind::ClaudeCode,
        AgentKind::Codex,
        AgentKind::Pi,
        AgentKind::Omp,
    ];

    /// The name used for `--agent <name>` and displayed to the user.
    pub fn slug(&self) -> &'static str {
        match self {
            AgentKind::Kiro => "kiro",
            AgentKind::KiroCli => "kiro-cli",
            AgentKind::OpenCode => "opencode",
            AgentKind::Pi => "pi",
            AgentKind::Omp => "omp",
            AgentKind::ClaudeCode => "claude",
            AgentKind::Codex => "codex",
        }
    }

    /// The binary name looked up on `PATH`. Distinct from `slug()` only where
    /// the CLI binary name differs from the friendly name.
    pub fn binary_name(&self) -> &'static str {
        match self {
            AgentKind::ClaudeCode => "claude",
            other => other.slug(),
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        Self::ALL.into_iter().find(|a| a.slug() == lower)
    }
}

/// A detected agent CLI, with the resolved binary path.
#[derive(Debug, Clone)]
pub struct DetectedAgent {
    pub kind: AgentKind,
    pub binary_path: PathBuf,
}

/// Look up every supported agent on `PATH` and return the ones found,
/// preserving `AgentKind::ALL` ordering.
pub fn detect_installed_agents() -> Vec<DetectedAgent> {
    AgentKind::ALL
        .into_iter()
        .filter_map(|kind| {
            which(kind.binary_name()).map(|path| DetectedAgent {
                kind,
                binary_path: path,
            })
        })
        .collect()
}

/// Minimal `which`-equivalent: search `PATH` for an executable named `name`.
fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() && is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> bool {
    true
}

/// Run `agent` headlessly with `prompt`, executed with `cwd` as the working
/// directory, and return its captured stdout.
///
/// `binary` is the executable to invoke: pass a `DetectedAgent::binary_path`
/// to pin to the exact binary that was found during detection (avoiding a
/// re-search of `PATH`, and any risk of it changing between detection and
/// invocation), or a bare name (e.g. from `--agent <name>` with no prior
/// detection) to let the OS resolve it via `PATH` at exec time.
///
/// Each agent's non-interactive contract differs; this centralizes the
/// per-agent flag mapping so callers only deal with a plain prompt string.
pub fn run_headless(
    agent: AgentKind,
    binary: &std::path::Path,
    prompt: &str,
    cwd: &std::path::Path,
) -> Result<String> {
    let mut command = build_command(agent, binary, prompt);
    command.current_dir(cwd);

    let output = command
        .output()
        .with_context(|| format!("failed to execute `{}`", agent.slug()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "`{}` exited with {}: {}",
            agent.slug(),
            output.status,
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn build_command(agent: AgentKind, binary: &std::path::Path, prompt: &str) -> Command {
    match agent {
        AgentKind::Kiro | AgentKind::KiroCli => {
            let mut cmd = Command::new(binary);
            cmd.arg("chat")
                .arg(prompt)
                .arg("--no-interactive")
                .arg("--trust-tools=fs_write,fs_read");
            cmd
        }
        AgentKind::OpenCode => {
            let mut cmd = Command::new(binary);
            cmd.arg("run").arg(prompt);
            cmd
        }
        AgentKind::ClaudeCode => {
            let mut cmd = Command::new(binary);
            cmd.arg("-p")
                .arg(prompt)
                .arg("--allowedTools")
                .arg("Write,Read");
            cmd
        }
        AgentKind::Codex => {
            let mut cmd = Command::new(binary);
            cmd.arg("exec").arg(prompt);
            cmd
        }
        AgentKind::Pi | AgentKind::Omp => {
            // No confirmed non-interactive contract for these at time of
            // writing; pass the prompt as a bare argument and let the
            // invocation fail loudly if that assumption is wrong, rather than
            // silently guessing wrong flags.
            let mut cmd = Command::new(binary);
            cmd.arg(prompt);
            cmd
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_roundtrips_through_parse() {
        for agent in AgentKind::ALL {
            assert_eq!(AgentKind::parse(agent.slug()), Some(agent));
        }
        assert_eq!(AgentKind::parse("not-a-real-agent"), None);
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(AgentKind::parse("KIRO-CLI"), Some(AgentKind::KiroCli));
    }
}
