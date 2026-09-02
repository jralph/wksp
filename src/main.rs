use anyhow::Result;
use clap::Parser;

use wksp::cli::{Cli, Commands, ShellTarget};
use wksp::commands;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let assume_yes = cli.yes;

    match cli.command {
        Commands::Find { repo } => commands::find::run(&repo),
        Commands::Go { repo } => commands::go::run(&repo),
        Commands::Move {
            org_repo,
            destination,
        } => commands::move_cmd::run(&org_repo, &destination, assume_yes),
        Commands::Get {
            org_repo,
            destination,
        } => commands::get::run(&org_repo, &destination, assume_yes),
        Commands::List { domain } => commands::list::run(domain.as_deref()),
        Commands::Make {
            domain_workspace,
            agent,
            domain_description,
            workspace_description,
        } => commands::make::run(
            &domain_workspace,
            agent.as_deref(),
            domain_description.as_deref(),
            workspace_description.as_deref(),
            assume_yes,
        ),
        Commands::Status { scope } => commands::status::run(scope.as_deref()),
        Commands::Orgs => commands::orgs::run(),
        Commands::Doctor => commands::doctor::run(),
        Commands::Init {
            path,
            agent,
            description,
        } => commands::init::run(path.as_deref(), agent.as_deref(), description.as_deref()),
        Commands::Shell { target } => match target {
            ShellTarget::Bash => commands::shell_setup::run_shell_integration("bash"),
            ShellTarget::Zsh => commands::shell_setup::run_shell_integration("zsh"),
            ShellTarget::Fish => commands::shell_setup::run_shell_integration("fish"),
            ShellTarget::Completions { shell } => commands::shell_setup::run_completions(shell),
        },
    }
}
