mod context;
mod extract;
mod install;
mod resources;
mod update;

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    version,
    about = "Source context, portable agent review guidance, and explicit release updates"
)]
struct Cli {
    /// Disable automatic version checks, cache writes, and update notifications.
    #[arg(long, global = true)]
    no_update_check: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Retrieve declared symbols or comments without executing project code.
    Context(context::Options),
    /// Retrieve authoritative review rules and their exceptions.
    Rules(resources::RuleOptions),
    /// Publish the bundled skill and this executable into a discovery directory.
    Install(install::Options),
    /// Check, preview, or explicitly apply a newer complete skill release.
    Update(update::Options),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let notify =
        !cli.no_update_check && matches!(&cli.command, Command::Context(_) | Command::Rules(_));
    let result = match cli.command {
        Command::Context(options) => context::run(options),
        Command::Rules(options) => resources::rules(options),
        Command::Install(options) => install::run(options),
        Command::Update(options) => update::run(options),
    };
    match result {
        Ok((output, code)) => {
            if let Err(error) = io::stdout().lock().write_all(output.as_bytes())
                && error.kind() != io::ErrorKind::BrokenPipe
            {
                eprintln!("Cannot write output: {error}");
                return ExitCode::from(2);
            }
            if notify {
                update::notify();
            }
            ExitCode::from(code)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
