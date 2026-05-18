//! Shell completion generation command.

use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionsArgs};

/// Generates shell completions and writes them to stdout.
pub fn handle(args: &CompletionsArgs) {
    let mut cmd = Cli::command();
    generate(
        args.shell,
        &mut cmd,
        "marketsurge-agent",
        &mut std::io::stdout(),
    );
}
