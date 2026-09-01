//! Library crate for `wksp`: a CLI for managing a domain/workspace/repo Git
//! workspace hierarchy (see the root `AGENTS.md` in `~/Workspaces` for the
//! convention this tool operates on).

pub mod agent;
pub mod cli;
pub mod commands;
pub mod interact;
pub mod shell;
pub mod workspace_tree;
