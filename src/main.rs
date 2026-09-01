use anyhow::Result;
use clap::Parser;

use wksp::cli::{Cli, Commands, InitTarget};
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
        Commands::Init { target } => match target {
            InitTarget::Bash => commands::init::run_shell_integration("bash"),
            InitTarget::Zsh => commands::init::run_shell_integration("zsh"),
            InitTarget::Fish => commands::init::run_shell_integration("fish"),
            InitTarget::Completions { shell } => commands::init::run_completions(shell),
        },
    }
}
