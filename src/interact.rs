//! Wraps `dialoguer` prompts so non-interactive callers (agents, scripts,
//! piped input) get a clear, actionable error instead of a raw "not a
//! terminal" IO error, and so `-y`/`--yes` can bypass yes/no confirmations.

use anyhow::{anyhow, Result};
use console::Term;
use dialoguer::{Confirm, Input, Select};

/// Whether stdin is an interactive terminal. Prompts should not be attempted
/// when this is false; callers should use `--yes` (for confirmations) or a
/// more specific query/flag (for selections) instead.
pub fn stdin_is_interactive() -> bool {
    Term::stdout().is_term() && Term::stderr().is_term()
}

/// A yes/no confirmation that respects `--yes` and fails clearly instead of
/// crashing when stdin isn't a terminal and `--yes` wasn't passed.
pub fn confirm(prompt: &str, assume_yes: bool, default: bool) -> Result<bool> {
    if assume_yes {
        return Ok(true);
    }

    if !stdin_is_interactive() {
        return Err(anyhow!(
            "`{prompt}` needs a yes/no answer, but stdin is not an interactive terminal. \
Re-run with -y/--yes to answer yes automatically."
        ));
    }

    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()?)
}

/// Ask the user to pick one of `labels`, failing clearly instead of crashing
/// when stdin isn't a terminal.
pub fn select(prompt: &str, labels: &[String], non_interactive_hint: &str) -> Result<usize> {
    if !stdin_is_interactive() {
        return Err(anyhow!(
            "`{prompt}` has more than one match and stdin is not an interactive terminal to ask which one. {non_interactive_hint}"
        ));
    }

    Ok(Select::new()
        .with_prompt(prompt)
        .items(labels)
        .default(0)
        .interact()?)
}

/// Ask the user for a line of free text, failing clearly instead of crashing
/// when stdin isn't a terminal.
pub fn input_text(prompt: &str, non_interactive_hint: &str) -> Result<String> {
    if !stdin_is_interactive() {
        return Err(anyhow!(
            "`{prompt}` needs a text answer and stdin is not an interactive terminal. {non_interactive_hint}"
        ));
    }

    Ok(Input::new().with_prompt(prompt).interact_text()?)
}
