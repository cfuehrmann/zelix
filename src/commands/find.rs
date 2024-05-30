use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, ExitCode},
};

use clap::Parser;
use tracing::{error, info};

use crate::{get_config, init_tracing, HiddenMethod, ProjectDir};

#[derive(Parser)]
pub struct FindArgs {
    #[clap(flatten)]
    project_dir: ProjectDir,

    /// The full path of the file to find
    #[clap(value_parser)]
    full_path_of_file: String,
}

pub fn find(
    FindArgs {
        project_dir: ProjectDir { project_dir },
        full_path_of_file,
    }: &FindArgs,
) -> Result<(), ExitCode> {
    let _guards = init_tracing(project_dir)?;

    info!(
        "Project dir is {}, file is {}",
        project_dir, full_path_of_file
    );

    let relative_path = get_relative_path(full_path_of_file, project_dir)?;

    let helix_chars = if relative_path.starts_with('.')
        && get_config(project_dir)?.find.hidden_method == HiddenMethod::Open
    {
        format!(":o {}", relative_path)
    } else {
        format!(" f{}", relative_path)
    };

    write_chars_to_helix(helix_chars)
}

fn get_relative_path<'a>(
    full_path_of_file: &'a str,
    project_dir: &str,
) -> Result<&'a str, ExitCode> {
    let path = Path::new(full_path_of_file);

    let relative_path = path.strip_prefix(project_dir).map_err(|e| {
        error!(
            "Failed to strip the prefix '{}' from the path '{}': {}",
            project_dir,
            path.display(),
            e
        );
        ExitCode::FAILURE
    })?;

    relative_path.to_str().ok_or_else(|| {
        error!(
            "Failed to convert the path '{}' to a string!",
            relative_path.display()
        );
        ExitCode::FAILURE
    })
}

fn write_chars_to_helix(chars: String) -> Result<(), ExitCode> {
    run_zellij_action(["write", "27"])?;
    run_zellij_action(["move-focus", "right"])?;
    run_zellij_action(["write", "27"])?;
    run_zellij_action(["write-chars", &chars])?;
    run_zellij_action(["write", "13"])
}

fn run_zellij_action<I, S>(args: I) -> Result<(), ExitCode>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("zellij");
    let command = command.arg("action").args(args);

    info!("Running '{:?}'", command);

    let output = command.output().map_err(|e| {
        error!("Command '{:?}' failed with error: {}", command, e);
        ExitCode::FAILURE
    })?;

    if !output.stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stdout = stdout.trim_end();
        info!("Command '{:?}' produced stdout: {}", command, stdout);
    }

    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim_end();
        error!("Command '{:?}' produced stderr: {}", command, stderr);
    }

    let exit_status = output.status;

    if exit_status.success() {
        info!("Command '{:?}' succeeded", command);
        Ok(())
    } else {
        error!("Command {:?} failed with status: {}", command, exit_status);
        Err(ExitCode::FAILURE)
    }
}
