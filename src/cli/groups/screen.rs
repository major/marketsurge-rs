//! Screen group: list saved screens, run screens, and ad-hoc queries.

use clap::Subcommand;

use crate::cli::ScreenArgs;
use crate::cli::commands;
use crate::cli::commands::adhoc_screen::AdhocScreenCommandArgs;
use crate::cli::commands::screen::{ListArgs, RunArgs};

/// Screen subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Run an ad-hoc screener query and return matching rows.
    #[command(
        after_help = "Examples:\n  marketsurge-agent screen adhoc --symbols AAPL,MSFT --columns Symbol,CompanyName,EPSRating\n  marketsurge-agent screen adhoc --screen-id 12345 --limit 100"
    )]
    Adhoc(AdhocScreenCommandArgs),

    /// List user screens, optionally including coach screens.
    #[command(
        after_help = "Examples:\n  marketsurge-agent screen list --coach\n  marketsurge-agent screen list --query ibd"
    )]
    List(ListArgs),

    /// Run a screen by ID or name and return matching instruments.
    #[command(
        after_help = "Examples:\n  marketsurge-agent screen run 'IBD 50'\n  marketsurge-agent screen run 'screen-Peter Lynch' --limit 250"
    )]
    Run(RunArgs),
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    match cmd {
        Cmd::Adhoc(args) => commands::adhoc_screen::handle(args, fields).await,
        Cmd::List(args) => {
            let screen_cmd = commands::screen::ScreenCommand::List(args.clone());
            commands::screen::handle(
                &ScreenArgs {
                    command: screen_cmd,
                },
                fields,
            )
            .await
        }
        Cmd::Run(args) => {
            let screen_cmd = commands::screen::ScreenCommand::Run(args.clone());
            commands::screen::handle(
                &ScreenArgs {
                    command: screen_cmd,
                },
                fields,
            )
            .await
        }
    }
}
