#![forbid(unsafe_code)]

use clap::Parser;
use gitctx_cli::{run, Cli};
use std::process;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
