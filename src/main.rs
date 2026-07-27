use std::process::ExitCode;

use v3::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse_normalized();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("v3: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Render(args) => v3::cli::execute_render(args),
        Command::Run(args) => v3::cli::execute_job(args),
        Command::Inspect(args) => v3::cli::execute_inspect(args),
        Command::Doctor(args) => v3::cli::execute_doctor(args),
    }
}
