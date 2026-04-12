//! # Twill Node
//!
//! Binary entry point for the Twill Network node.
//! Standalone L1 with permissionless block production (instant-seal).
//! No authority keys. No Aura. No GRANDPA. Tethered to no one.

mod chain_spec;
mod command;
mod rpc;
mod service;

fn main() -> sc_cli::Result<()> {
    command::run()
}
