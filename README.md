# wksp

A CLI for managing a local `domain -> workspace -> repo` Git workspace hierarchy under `~/Workspaces`:

```
~/Workspaces/<domain>/<workspace>/<repo>
```

- **domain** — a broad area, e.g. `platform`, `aws`, `personal`.
- **workspace** — a cohesive group of repos sharing context/tooling, e.g. `customer-identity`.
- **repo** — a single Git checkout, e.g. `customer-service`.

A repo's org/owner is never inferred from directory names — it's read from the repo's `origin` remote, so the same tree can mix repos from different GitHub orgs (e.g. `gymshark/customer-service`, `jralph/wksp`).

## Install

```bash
make install
```

Builds the release binary and symlinks it into `~/.local/bin/wksp` (rebuilding later and re-running `make install` updates the same symlink). Warns if `~/.local/bin` isn't on your `PATH`. `make uninstall` removes it.

Alternatively, without the Makefile:

```bash
cargo install --path .
```

or build a release binary directly:

```bash
cargo build --release
# binary at target/release/wksp
```

Run `make help` for every available target (`build`, `install`, `uninstall`, `clean`, `test`, `fmt`, `lint`).

### Shell integration (optional, recommended)

`wksp` is a plain subprocess and can't change your shell's working directory by itself. `wksp go <repo>` prints the resolved path; install the shell wrapper once so it actually `cd`s for you:

```bash
# ~/.zshrc or ~/.bashrc
eval "$(wksp init zsh)"   # or: bash / fish
```

Tab completions:

```bash
eval "$(wksp init completions zsh)"
```

## Commands

Every command has full `--help` text with examples — run `wksp --help` or `wksp <command> --help`. Summary:

| Command | Purpose |
| --- | --- |
| `wksp find <repo>` | Locate repos. `org/` lists an org, `repo` substring-matches by name, `org/repo` is an exact lookup. |
| `wksp go <repo>` | Same resolution as `find`, but `cd`s into the matched repo's workspace (requires shell integration). |
| `wksp move <org>/<repo> <domain>/<workspace>` | Move an existing checkout to a different domain/workspace on disk. |
| `wksp get <org>/<repo> <domain>/<workspace>` | Clone a repo into a domain/workspace, or offer to move it there if already checked out elsewhere. |
| `wksp list [domain]` | List domains/workspaces/repos, optionally scoped to one domain. |
| `wksp make <domain>/<workspace>` | Scaffold a domain/workspace's `AGENTS.md`/`README.md` via a coding-agent CLI. |
| `wksp status [scope]` | Git status (clean/dirty, ahead/behind) summary across matching repos. |
| `wksp orgs` | List every distinct org found across checked-out repos. |
| `wksp doctor` | Validate the tree against the documented convention (empty workspaces, worktree shape, repos with no resolvable org). |
| `wksp init <shell>` | Shell integration or tab-completion scripts. |

`move` and `get` accept `-y`/`--yes` to auto-confirm prompts (useful when driving `wksp` from a script or agent). When stdin isn't an interactive terminal, prompts that would otherwise hang instead fail with an explicit message telling you which flag to pass.

## `wksp make` and agent CLIs

`make` delegates writing `AGENTS.md`/`README.md` content to a coding-agent CLI, invoked headlessly (no interactive session). Supported out of the box: `kiro`, `kiro-cli`, `opencode`, `claude` (Claude Code), `codex`, `pi`, `omp`. If more than one is detected on `PATH` you'll be asked which to use; pass `--agent <name>` to skip that.

## Testing

```bash
cargo test
cargo clippy --all-targets
```

or `make test` / `make lint` (the latter treats warnings as errors).

Override the tree root for testing or an alternate hierarchy via `WKSP_ROOT`:

```bash
WKSP_ROOT=/tmp/scratch-workspaces wksp list
```

## License

MIT — see [LICENSE](./LICENSE).
