use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use tracing::{error, info};

use crate::{config::Config, init_tracing, ProjectDir};

pub fn open(ProjectDir { project_dir }: &ProjectDir) -> Result<(), ExitCode> {
    let _guards = init_tracing(project_dir)?;

    info!("Project dir is {}", project_dir);

    let config = Config::load(project_dir)?;

    let session = config.session.ok_or_else(|| {
        error!("No session is specified in the configuration file!");
        ExitCode::FAILURE
    })?;

    info!("Session is '{}'", session);

    delete_session(&session)?;

    let zellij_config_dir = get_zellij_config_dir(project_dir)?;
    let zellij_args = ["--session", &session, "--config-dir", &zellij_config_dir];
    let terminal = config.open.terminal;
    info!("Terminal is {:?}", terminal);

    env::set_current_dir(project_dir).map_err(|e| {
        error!(
            "Failed to set the current directory to '{}': {}",
            project_dir, e
        );
        ExitCode::FAILURE
    })?;

    // If `open.terminal` is not set in the configuration files, it is unwrapped
    // to the default []. So the first match arm will be used.
    match terminal.unwrap_or_default().as_slice() {
        [] => run_zellij_and_wait(zellij_args),
        [terminal_command, terminal_args @ ..] => {
            spawn_zellij_in_terminal(terminal_command, terminal_args, zellij_args)
        }
    }
}

fn delete_session(session: &str) -> Result<(), ExitCode> {
    let mut command = Command::new("zellij");
    let command = command.args(["delete-session", session]);

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

        // This does not necessarily indicate an error. For example, it could
        // be that there is no session to delete
        info!("Command '{:?}' produced stderr: {}", command, stderr);
    }

    let exit_status = output.status;

    match exit_status.code() {
        Some(0) => {
            info!("The exited session was deleted: {}", session);
            Ok(())
        }
        Some(1) => {
            error!("Cannot delete running session: {}", session);
            Err(ExitCode::FAILURE)
        }
        Some(2) => {
            info!("There was no session to delete: {}", session);
            Ok(())
        }
        _ => {
            error!("Command '{:?}' failed with {}", command, exit_status);
            Err(ExitCode::FAILURE)
        }
    }
}

fn get_zellij_config_dir(project_dir: &str) -> Result<String, ExitCode> {
    let dir = PathBuf::from(project_dir)
        .join("zelix-config")
        .join("zellij");

    let exists = Path::new(&dir).try_exists().map_err(|e| {
        error!("Failed to check if directory '{:?}' exists: {}", dir, e);
        ExitCode::FAILURE
    })?;

    let dir = if exists {
        info!("Directory exists: {:?}", dir);
        dir
    } else {
        info!("Directory does not exist: {:?}", dir);

        let home = env::var("HOME").map_err(|e| {
            error!("Failed to read $HOME: {}", e);
            ExitCode::FAILURE
        })?;

        let dir = PathBuf::from(home)
            .join(".config")
            .join("zelix")
            .join("zellij");

        let exist = Path::new(&dir).try_exists().map_err(|e| {
            error!("Failed to check if directory '{:?}' exists: {}", dir, e);
            ExitCode::FAILURE
        })?;

        if exist {
            info!("Directory exists: {:?}", dir);
            dir
        } else {
            error!("Directory does not exist: {:?}", dir);
            return Err(ExitCode::FAILURE);
        }
    };

    let dir = fs::canonicalize(&dir)
        .map_err(|e| {
            error!("Failed to canonicalize the directory '{:?}': {}", dir, e);
            ExitCode::FAILURE
        })?
        .to_str()
        .ok_or_else(|| {
            error!("Failed to convert path to a string: {:?}", dir);
            ExitCode::FAILURE
        })?
        .to_owned();

    Ok(dir)
}

fn run_zellij_and_wait(zellij_args: [&str; 4]) -> Result<(), ExitCode> {
    let mut command = Command::new("zellij");
    let command = command.args(zellij_args);

    // Don't capture output here, since that keeps the zellij UI from
    // appearing
    let mut child = command.spawn().map_err(|e| {
        error!("Failed to spawn '{:?}': {}", command, e);
        ExitCode::FAILURE
    })?;

    let pid = child.id();

    info!("Spawned process {} from command '{:?}'", pid, command);

    let exit_status = child.wait().map_err(|e| {
        // I don't know if this can happen
        error!(
            "Failed to wait for process {} from command '{:?}': {}",
            pid, command, e
        );
        ExitCode::FAILURE
    })?;

    if exit_status.success() {
        info!("Process {} from command '{:?}' succeeded", pid, command);
        Ok(())
    } else {
        error!(
            "Process {} from command '{:?}' failed with status: {}",
            pid, command, exit_status
        );
        Err(ExitCode::FAILURE)
    }
}

fn spawn_zellij_in_terminal(
    terminal_command: &str,
    terminal_args: &[String],
    zellij_args: [&str; 4],
) -> Result<(), ExitCode> {
    let mut command = Command::new(terminal_command);
    let command = command.args(terminal_args).arg("zellij").args(zellij_args);

    command
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn '{:?}': {}", command, e);
            ExitCode::FAILURE
        })
        .map(|child| {
            let pid = child.id();
            info!("Spawned process {} from command '{:?}'", pid, command);
        })
}
