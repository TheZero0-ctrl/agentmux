//! Agentmux library crate.

#![forbid(unsafe_code)]

/// Application state and lifecycle.
pub mod app;
/// Command-line interface.
pub mod cli;
/// Daemon-owned discovery facade.
pub mod daemon;
/// Key input translation.
pub mod input;
/// One-shot tmux-derived inspect output.
pub mod inspect;
/// Typed discovery model.
pub mod model;
/// Generic process normalization.
pub mod process;
/// Shared projection from discovery state to presentation rows.
pub mod projection;
/// Dashboard runner boundary.
pub mod runner;
/// Deterministic state normalization.
pub mod state;
/// tmux command boundary and parser.
pub mod tmux;
/// Ratatui rendering.
pub mod tui;
