//! Generic process normalization for fallback discovery.

mod classifier;
mod command;
/// Linux procfs process-tree collection primitives.
pub mod procfs;

#[cfg(test)]
mod procfs_test_support;
#[cfg(test)]
mod procfs_tests;

pub use classifier::{classify_process, classify_process_tree};
pub use command::normalize_command_name;
