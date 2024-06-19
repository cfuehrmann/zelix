use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, ExitCode},
};

use clap::Parser;
use tracing::{error, info, warn};

use crate::{
    config::{Config, HiddenMethod},
    init_tracing, ProjectDir,
};

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

    let helix_chars = match is_hidden(full_path_of_file) {
        Some(true) => {
            let method = Config::load(project_dir)?.find.hidden_method;

            info!(
                "The file is hidden. The configured method for hidden files is {:?}",
                method
            );

            if method != Some(HiddenMethod::Picker) {
                // If method is None, then `find.hidden_method` is not set in the
                // configuration files. In that case, we use ":o", because that way
                // the file selected by the user is guaranteed to be found.
                format!(":o {}", relative_path)
            } else {
                // We only use " f" if the method is Picker. This only makes sense
                // when Helix is configured to show hidden files on " f".
                format!(" f{}", relative_path)
            }
        }
        Some(false) => {
            info!("The file is not hidden.");

            // Non-hidden files are found by " f". So " f" is better than
            // ": o", because " f" preserves the cursor position.
            format!(" f{}", relative_path)
        }
        None => {
            warn!("It is unknown if the file is hidden.");

            // In this case we use ": o", because it seem to be the most
            // reliable way to find the file.
            format!(":o {}", relative_path)
        }
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
            "Failed to strip the prefix '{}' from the path '{:?}': {}",
            project_dir, path, e
        );
        ExitCode::FAILURE
    })?;

    relative_path.to_str().ok_or_else(|| {
        error!(
            "Failed to convert the path '{:?}' to a string!",
            relative_path
        );
        ExitCode::FAILURE
    })
}

fn is_hidden(file_path: &str) -> Option<bool> {
    Path::new(file_path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(|file_name_str| file_name_str.starts_with('.'))
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
