pub mod clone_references;
pub mod eval;
pub mod fixtures;
pub mod lint;

use anyhow::Result;

#[allow(clippy::unnecessary_wraps)]
pub fn stub_command(name: &str) -> Result<()> {
    println!("xtask subcommand '{name}' will be implemented in subsequent phase 1 plans.");
    Ok(())
}
