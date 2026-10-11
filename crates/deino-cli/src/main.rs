//! `deino`: parses the command line, runs it, prints the result and exits
//! with the failure's code (see `CliError::exit_code`).

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use deino_cli::{Cli, CliError, execute};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build();
    let outcome = match runtime {
        Ok(runtime) => runtime.block_on(execute(cli)),
        Err(error) => Err(CliError::Start(error)),
    };
    match outcome {
        Ok(text) => print(&text),
        Err(error) => fail(&error, json),
    }
}

/// Prints `text`; a closed pipe (`deino … | head`) is not a failure.
fn print(text: &str) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Reports `error` on stderr, as JSON with `--json`.
fn fail(error: &CliError, json: bool) -> ExitCode {
    let report = if json {
        serde_json::json!({ "error": error.body() }).to_string()
    } else {
        format!("error: {error}")
    };
    let mut stderr = std::io::stderr().lock();
    // Nothing else can report a failure to write the failure.
    if writeln!(stderr, "{report}").is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::from(error.exit_code())
}
