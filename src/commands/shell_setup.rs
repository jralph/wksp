//! `wksp shell <shell>` and `wksp shell completions <shell>`

use anyhow::{anyhow, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell as ClapShell};

use crate::cli::Cli;
use crate::shell::{render_init_script, Shell};

pub fn run_shell_integration(shell_name: &str) -> Result<()> {
    let shell = Shell::parse(shell_name)
        .ok_or_else(|| anyhow!("unsupported shell `{shell_name}` (expected bash, zsh, or fish)"))?;
    print!("{}", render_init_script(shell));
    Ok(())
}

pub fn run_completions(shell: ClapShell) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}
