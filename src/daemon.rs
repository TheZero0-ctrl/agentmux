//! Live daemon, local API, and discovery facade.

pub mod api;
pub mod autostart;
pub mod client;
mod discovery;
pub mod evidence;
pub mod runner;
pub mod state;

pub use discovery::{DiscoveryService, ProcessTreeEvidence};
