mod commands;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use commands::{find, open, FindArgs};
use serde::Deserialize;
use tracing::{error, level_filters::LevelFilter};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Open a project
    Open(ProjectDir),

    /// Find a file in the project and show it in Helix
    Find(FindArgs),
}

#[derive(Parser)]
struct ProjectDir {
    /// The project directory
    #[clap(value_parser)]
    project_dir: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    session: String,
    open: OpenSection,
    find: FindSection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct OpenSection {
    terminal: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct FindSection {
    hidden_method: HiddenMethod,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum HiddenMethod {
    Open,
    Picker,
}

fn main() -> ExitCode {
    match &Cli::parse().command {
        Some(Command::Open(args)) => open(args),
        Some(Command::Find(args)) => find(args),
        None => {
            eprintln!("No subcommand given.");
            Err(ExitCode::FAILURE)
        }
    }
    .map_or_else(|exit_code| exit_code, |()| ExitCode::SUCCESS)
}

fn init_tracing(project_dir: &str) -> Result<(WorkerGuard, WorkerGuard), ExitCode> {
    let dir = PathBuf::from(project_dir).join("zelix-config");
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

fn get_config(project_dir: &str) -> Result<Config, ExitCode> {
    let path = PathBuf::from(project_dir)
        .join("zelix-config")
        .join("config.toml");

    let mut file = File::open(&path).map_err(|e| {
        error!("Failed to open the file '{}': {}", path.display(), e);
        ExitCode::FAILURE
    })?;

    let mut buf = String::new();

    file.read_to_string(&mut buf).map_err(|e| {
        error!("Failed to read the file '{}': {}", path.display(), e);
        ExitCode::FAILURE
    })?;

    toml::from_str(&buf).map_err(|e| {
        error!("Failed to parse the file '{}': {}", path.display(), e);
        ExitCode::FAILURE
    })
}
