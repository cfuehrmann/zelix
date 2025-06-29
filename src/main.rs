mod commands;
mod config;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use commands::{FindInHelixArgs, OpenInHelixArgs, find, find_in_helix, open, open_in_helix};
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Open a Zelix project
    Open(ProjectDir),

    /// Use Helix' file picker (Space f) on a file
    FindInHelix(FindInHelixArgs),

    /// Legacy name for the find-in-helix command
    Find(FindInHelixArgs),

    /// Use Helix' open command (:open) on a file
    OpenInHelix(OpenInHelixArgs),
}

#[derive(Parser)]
struct ProjectDir {
    /// The project directory
    #[clap(value_parser)]
    project_dir: String,
}

fn main() -> ExitCode {
    match &Cli::parse().command {
        Some(Command::Open(args)) => open(args),
        Some(Command::FindInHelix(args)) => find_in_helix(args),
        Some(Command::Find(args)) => find(args),
        Some(Command::OpenInHelix(args)) => open_in_helix(args),
        None => {
            eprintln!("No subcommand given.");
            Err(ExitCode::FAILURE)
        }
    }
    .map_or_else(|exit_code| exit_code, |()| ExitCode::SUCCESS)
}

fn init_tracing(project_dir: &str) -> Result<(WorkerGuard, WorkerGuard), ExitCode> {
    let dir = PathBuf::from(project_dir);
    let file_appender = tracing_appender::rolling::daily(dir, "zelix.log");
    let (non_blocking_file, file_guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer().with_writer(non_blocking_file).with_ansi(false); // Disable ANSI colors for file logging

    let (non_blocking_stderr, stderr_guard) = tracing_appender::non_blocking(std::io::stderr());
    let stderr_layer = fmt::layer()
        .with_writer(non_blocking_stderr)
        // Suppress ANSI codes because mesages might be shown in situations
        // where they are not interpreted as color
        .with_ansi(false)
        .without_time()
        .with_filter(LevelFilter::ERROR);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .try_init()
        .map_err(|e| {
            eprintln!("Failed to initialize the tracing subscriber: {}", e);
            ExitCode::FAILURE
        })?;

    Ok((file_guard, stderr_guard)) // Tracing flushes when the caller drops the guards
}
