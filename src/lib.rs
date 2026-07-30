//! Agentmux library crate.

#![forbid(unsafe_code)]

/// Application state and lifecycle.
pub mod app;
/// Command-line interface.
pub mod cli;
/// Key input translation.
pub mod input;
/// Dashboard runner boundary.
pub mod runner;
/// Ratatui rendering.
pub mod tui;
