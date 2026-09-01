//! Command-line argument definitions for `wksp`.
//!
//! Every command and subcommand carries a `long_about` with enough detail
//! (including examples) that an agent or a first-time user can drive this
//! tool correctly from `--help` alone, without reading external docs.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "wksp",
    version,
    about = "Manage the ~/Workspaces domain/workspace/repo hierarchy",
    long_about = "\
wksp manages a local Git workspace hierarchy structured as:

    ~/Workspaces/<domain>/<workspace>/<repo>

  - domain:    a broad area, e.g. `platform`, `aws`, `personal`.
  - workspace: a cohesive group of repos with shared context, e.g. `customer-identity`.
  - repo:      a single Git checkout, e.g. `customer-service`.

A repo's GitHub org/owner is never inferred from directory names — it is read \
from the repo's `origin` git remote, so the same directory tree can mix repos \
from different orgs (e.g. `gymshark/customer-service`, `jralph/wksp`).

The tree root defaults to `~/Workspaces` and can be overridden for testing or \
alternate hierarchies via the WKSP_ROOT environment variable.

Non-interactive use (e.g. from an agent or a script): commands that would \
otherwise ask a yes/no question (`move`, `get`) accept `-y`/`--yes` to answer \
yes automatically. Commands that would ask you to pick from several matches \
(`go` with an ambiguous query, `make` with multiple agents detected) fail \
with a clear error telling you how to be more specific instead of hanging, \
whenever stdin is not an interactive terminal.

Run `wksp <command> --help` for full details and examples on any command.",
    propagate_version = true
)]
pub struct Cli {
    /// Assume "yes" for any yes/no confirmation prompt (move/get). Has no
    /// effect on commands that ask you to pick one of several matches.
    #[arg(short = 'y', long, global = true)]
    pub yes: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Find one or more repos by org, name, or org/repo.
    #[command(long_about = "\
Find one or more repos by org, name, or org/repo, and print their locations.

REPO accepts three forms, disambiguated by shape (no guessing whether a token \
is a known org):

  1. `<org>/`      A trailing slash explicitly lists every repo checked out \
under that org, grouped by domain/workspace.
  2. `<repo>`       No slash: REPO is a case-insensitive substring match \
against repo names, across every org and domain/workspace.
  3. `<org>/<repo>` An exact lookup for one specific repo under one specific org.

Examples:
  wksp find gymshark/                 # every gymshark repo, by location
  wksp find customer-service          # any repo with 'customer-service' in its name
  wksp find gymshark/customer-service # the exact repo, if checked out
")]
    Find {
        /// Org name with trailing '/', repo name (substring), or org/repo.
        repo: String,
    },

    /// Resolve a repo and print its workspace directory (for `cd`).
    #[command(long_about = "\
Resolve REPO using the same rules as `wksp find` (`<org>/` lists every repo in \
an org, `<repo>` substring-matches by name, `<org>/<repo>` is an exact lookup), \
then print the *workspace* directory containing the matched repo (not the \
repo directory itself) so a wrapping shell function can `cd` into it.

wksp cannot change your shell's working directory directly — it is a plain \
subprocess. Install shell integration once with:

  eval \"$(wksp init zsh)\"    # or: wksp init bash / wksp init fish

after which `wksp go <repo>` in your shell will actually change directory.
Without the shell integration, this command still works but only prints the \
resolved path to stdout; use it as `cd \"$(wksp go <repo>)\"`.

Resolution behavior:
  - Exactly one match: prints its workspace path and exits 0.
  - Multiple matches (e.g. a bare name substring, or an org with several \
repos): interactively asks which one you meant, then prints it.
  - No matches: prints an error to stderr and exits non-zero.

Examples:
  wksp go customer-service
  wksp go gymshark/customer-service
  cd \"$(wksp go customer-service)\"   # without shell integration installed
")]
    Go {
        /// Org name with trailing '/', repo name (substring), or org/repo.
        repo: String,
    },

    /// Move a checked-out repo to a different domain/workspace.
    #[command(long_about = "\
Move (not copy) an existing repo checkout to a different domain/workspace on \
disk. This is a filesystem move: the repo keeps its Git history, remotes, and \
worktrees; only its location under ~/Workspaces changes.

ORG_REPO must be the exact `org/repo` form (unlike `find`/`go`, no fuzzy \
matching is done here, since a move should target exactly one repo).

DESTINATION is `domain/workspace`. If either the domain or workspace doesn't \
exist yet, you're asked whether to create it; accepting runs `wksp make` for \
you so the new domain/workspace gets a real AGENTS.md and README.md (not bare \
directories) before the repo is moved in. Declining cancels the move. Pass \
-y/--yes to accept automatically (e.g. when calling this from a script or agent).

Fails without changing anything if:
  - ORG_REPO does not resolve to a checked-out repo.
  - a repo of the same name already exists at the destination workspace.

Examples:
  wksp move gymshark/customer-service platform/customer-identity
  wksp move jralph/wksp personal/workspace-manager
  wksp move -y jralph/wksp personal/workspace-manager   # don't prompt
")]
    Move {
        /// Exact `org/repo` of the checked-out repo to move.
        org_repo: String,
        /// Destination as `domain/workspace`.
        destination: String,
    },

    /// Clone a repo into a domain/workspace via `git clone`.
    #[command(long_about = "\
Clone ORG_REPO via `git clone` into DESTINATION (`domain/workspace`).

If either the destination domain or workspace doesn't exist yet, you're asked \
whether to create it; accepting runs `wksp make` for you (real AGENTS.md and \
README.md, not bare directories) before cloning. Declining cancels. Pass \
-y/--yes to accept automatically (e.g. when calling this from a script or agent).

Before cloning, checks whether ORG_REPO is already checked out anywhere in the \
tree:

  - Already checked out at exactly DESTINATION: prints a message and exits 0 \
(no-op; nothing is cloned or changed).
  - Already checked out somewhere else: asks whether to move the existing \
checkout to DESTINATION instead of cloning a duplicate (also affected by \
-y/--yes). Declining exits without cloning.
  - Not checked out anywhere: clones fresh into DESTINATION.

Uses SSH clone URLs (`git@github.com:org/repo.git`) by default, matching the \
convention used across this workspace tree.

Examples:
  wksp get gymshark/customer-service platform/customer-identity
  wksp get jralph/wksp personal/workspace-manager
")]
    Get {
        /// Exact `org/repo` to clone or locate.
        org_repo: String,
        /// Destination as `domain/workspace`.
        destination: String,
    },

    /// List domains, workspaces, and repos.
    #[command(long_about = "\
List the workspace hierarchy.

With no argument, lists every domain along with its workspaces and how many \
repos each workspace contains.

With DOMAIN given, lists that domain's workspaces and every repo (with its \
org) inside each one.

Examples:
  wksp list                 # every domain and workspace, repo counts
  wksp list platform        # every workspace and repo under 'platform'
")]
    List {
        /// Optional domain to list in detail.
        domain: Option<String>,
    },

    /// Scaffold a domain/workspace with AGENTS.md and README.md via an agent.
    #[command(long_about = "\
Create DOMAIN and/or WORKSPACE (whichever don't already exist) and generate \
their AGENTS.md and README.md files by delegating to a coding-agent CLI.

DOMAIN_WORKSPACE is `domain/workspace`. For each missing AGENTS.md/README.md, \
you'll be prompted for a short description of the domain and/or workspace \
purpose (only for pieces that are actually missing or being created) unless \
--domain-description / --workspace-description are given up front.

Agent selection:
  --agent <name>   use this agent without prompting (kiro, kiro-cli, opencode, \
claude, codex, pi, omp).
  (omitted)        if exactly one supported agent is detected on PATH, it is \
used silently; if more than one is detected, you are asked to pick; if none \
are detected, this command exits with an error explaining how to install one.

The chosen agent is invoked headlessly (no interactive session) with a prompt \
built from your description plus the existing parent AGENTS.md content for \
style consistency, and is instructed to write the files itself. If the agent \
does not end up creating a file, its raw response text is written to that \
file as a fallback so the command never silently does nothing.

Examples:
  wksp make personal/workspace-manager
  wksp make platform/new-service --agent kiro-cli
  wksp make aws/new-area --domain-description \"New AWS area for X\" \\
                          --workspace-description \"Repos supporting Y\"
")]
    Make {
        /// Target as `domain/workspace`.
        domain_workspace: String,
        /// Agent CLI to use (kiro, kiro-cli, opencode, claude, codex, pi, omp).
        #[arg(long)]
        agent: Option<String>,
        /// Description of the domain's purpose, if the domain is being created.
        #[arg(long = "domain-description")]
        domain_description: Option<String>,
        /// Description of the workspace's purpose, if it's being created.
        #[arg(long = "workspace-description")]
        workspace_description: Option<String>,
    },

    /// Print a git status summary for repos matching a domain/workspace.
    #[command(long_about = "\
Print a one-line git status summary (clean/dirty, ahead/behind origin) for \
every repo under SCOPE.

SCOPE is optional and may be a bare domain (`platform`) or `domain/workspace` \
(`platform/customer-identity`). With no SCOPE, every indexed repo is checked.

Examples:
  wksp status
  wksp status platform
  wksp status platform/customer-identity
")]
    Status {
        /// Optional `domain` or `domain/workspace` to scope the check to.
        scope: Option<String>,
    },

    /// List every distinct org found across checked-out repos.
    #[command(long_about = "\
List every distinct GitHub org/owner detected across all checked-out repos \
(parsed from each repo's `origin` remote), with a count of repos per org.

Example:
  wksp orgs
")]
    Orgs,

    /// Validate the workspace tree against the documented conventions.
    #[command(long_about = "\
Check the workspace tree for structural drift from the documented convention \
(domain -> workspace -> repo, worktrees only under .worktrees, no empty \
workspaces) and print a report. Does not modify anything.

Flags:
  - workspaces with zero repos and zero AGENTS.md,
  - non-worktree directories found inside a `.worktrees` directory,
  - domains that contain files/dirs other than workspaces, AGENTS.md, README.md.

Example:
  wksp doctor
")]
    Doctor,

    /// Print shell integration for `wksp go`, or generate shell completions.
    #[command(long_about = "\
Print a shell snippet to stdout. Two independent uses:

  wksp init <shell>          shell integration enabling `wksp go` to actually \
`cd` your shell (bash, zsh, fish). Add to your shell rc file:
                                eval \"$(wksp init zsh)\"

  wksp init completions <shell>  clap-generated tab completions for wksp \
itself (bash, zsh, fish, elvish, powershell). Add similarly:
                                eval \"$(wksp init completions zsh)\"

Examples:
  eval \"$(wksp init zsh)\"
  eval \"$(wksp init completions zsh)\"
")]
    Init {
        #[command(subcommand)]
        target: InitTarget,
    },
}

#[derive(Subcommand, Debug)]
pub enum InitTarget {
    /// Shell integration so `wksp go` can `cd` your shell.
    Bash,
    /// Shell integration so `wksp go` can `cd` your shell.
    Zsh,
    /// Shell integration so `wksp go` can `cd` your shell.
    Fish,
    /// Generate clap tab-completion scripts for wksp itself.
    Completions {
        /// Shell to generate completions for.
        shell: clap_complete::Shell,
    },
}
