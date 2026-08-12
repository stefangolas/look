use std::process::ExitCode;

use look::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse_normalized();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("look: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Render(args) => look::cli::execute_render(*args),
        Command::Ui(args) => look::cli::execute_ui(args),
        Command::Run(args) => look::cli::execute_job(args),
        Command::Inspect(args) => look::cli::execute_inspect(args),
        Command::Doctor(args) => look::cli::execute_doctor(args),
        Command::Persist(args) => look::cli::execute_persist(args),
        Command::Sessions(args) => look::cli::execute_sessions(args),
        Command::Close(args) => look::cli::execute_close(args),
        Command::Server(args) => look::cli::execute_server_command(args),
        Command::Serve => look::cli::execute_server_daemon(),
    }
}
