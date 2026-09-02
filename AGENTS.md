# AGENTS.md

This file follows the open [agents.md](https://agents.md/) standard — a "README for agents." It complements `README.md`, which stays focused on human usage.

## Project overview

`wksp` is a Rust CLI for managing a local Git workspace hierarchy structured as:

```
~/Workspaces/<domain>/<workspace>/<repo>
```

A repo's GitHub org/owner is never inferred from directory names — `workspace_tree.rs` reads it from the repo's `origin` git remote via `git remote get-url origin`, so the same tree can mix repos from different orgs. The tree root defaults to `~/Workspaces` and can be overridden with the `WKSP_ROOT` environment variable (used throughout the test suite instead of mutating real global state).

## Command reference

Every command's authoritative documentation is its own `--help`/`long_about` text in `src/cli.rs` — read that before changing behavior, and update it in the same change if behavior changes. Current surface:

| Command | File |
| --- | --- |
| `find <repo>` | `src/commands/find.rs` |
| `go <repo>` | `src/commands/go.rs` |
| `move <org>/<repo> <domain>/<workspace>` | `src/commands/move_cmd.rs` |
| `get <org>/<repo> <domain>/<workspace>` | `src/commands/get.rs` |
| `list [domain]` | `src/commands/list.rs` |
| `make <domain>/<workspace>` | `src/commands/make.rs` |
| `status [scope]` | `src/commands/status.rs` |
| `orgs` | `src/commands/orgs.rs` |
| `doctor` | `src/commands/doctor.rs` |
| `init [--path <dir>]` | `src/commands/init.rs` |
| `shell <shell>` / `shell completions <shell>` | `src/commands/shell_setup.rs` |

`init` and `shell` are deliberately separate commands, not subcommands of one another: `init` bootstraps the hierarchy *root* (creates `~/Workspaces` itself, or `--path <dir>`, plus its `AGENTS.md`/`README.md` — no domains/workspaces/repos), while `shell` prints shell-integration/completion snippets. They used to share the `init` name (`shell`'s content lived at `wksp init <shell>`) before the root-bootstrap command was added; if you see a stale `wksp init zsh`-style reference anywhere outside this repo, it means `wksp shell zsh`.

`make` and `init` share their agent-invocation logic (`src/scaffold.rs`): pick an agent CLI (`--agent`, or detect/prompt among `kiro`, `kiro-cli`, `opencode`, `claude`, `codex`, `pi`, `omp` on `PATH`), build a level-appropriate prompt, run it headlessly via `src/agent.rs`, and fall back to writing the agent's raw response if it didn't create the requested file(s) itself. Adding a new "generate documentation for X" command should reuse `scaffold.rs`, not duplicate this.

## Adding or changing a command

1. Add/edit the `clap` variant in `src/cli.rs`, including a full `long_about` with usage examples — this is the tool's only user-facing documentation surface, so it must be complete on its own.
2. Implement it in `src/commands/<name>.rs`, dispatched from `src/main.rs`.
3. Update the command table in **both** `README.md` and this file (`AGENTS.md`).
4. Prefer non-interactive-safe prompting: use `src/interact.rs`'s `confirm`/`select`/`input_text` wrappers, not raw `dialoguer` calls — they detect a non-terminal stdin and fail with an actionable message instead of crashing, and `confirm` respects the global `-y`/`--yes` flag. This matters because `wksp` is meant to be drivable by an agent, not just a human at a TTY.
5. Add unit tests. `workspace_tree::discover_at`/`WKSP_ROOT` let tests build a scratch tree without touching `~/Workspaces` or mutating process-global env state where avoidable.

## Build, test, lint

```bash
cargo build --release      # or: make build
cargo test                 # or: make test
cargo clippy --all-targets -- -D warnings   # or: make lint
cargo fmt --check           # or: make fmt
```

All four must be clean before committing. `make install` builds and symlinks the binary into `~/.local/bin/wksp`; `make help` lists every target.

## Verifying behavior changes

Prefer a scratch `WKSP_ROOT` over the real `~/Workspaces` tree when manually exercising a change:

```bash
WKSP_ROOT=$(mktemp -d) cargo run --release -- list
```

Commands that shell out to `git` (`move`, `get`, `status`) or an agent CLI (`make`, `init`) are worth a real end-to-end run against a scratch root, not just unit tests, since the unit tests don't cover subprocess behavior. Headless agent invocations (`kiro-cli chat --no-interactive`, `opencode run`, etc.) can take 40s–2min depending on the agent's own startup/MCP warm-up — that's the agent CLI's overhead, not `wksp`'s; don't reduce timeouts to "fix" this.

## Security and safety considerations

- `move`/`get` never overwrite an existing repo checkout at the destination — they refuse and exit non-zero instead.
- `get` clones via SSH URLs (`git@github.com:org/repo.git`); it doesn't currently support HTTPS clone URLs as a destination scheme (only as something `parse_org_from_remote` can read *from* an existing checkout).
- `make`/`init`'s fallback (writing an agent's raw response directly to a file when it didn't write the file itself) only fires when *exactly one* requested file is still missing — it deliberately never copies one file's content into a differently-purposed file when both are missing, to avoid writing garbage into, e.g., `README.md` using text meant for `AGENTS.md`.
- A repo with no resolvable `origin` remote is only reachable via `find`/`go`'s bare-name substring match — never via `org/repo`, `move`, or `get`. `wksp doctor` flags these; see `check_unattributed_repos` in `src/commands/doctor.rs`.

## Further reading

- [README.md](README.md) — full command reference and install instructions for human use.
- The root `~/Workspaces/AGENTS.md` (or wherever `wksp init` created it) — the domain/workspace/repo convention this tool implements.
