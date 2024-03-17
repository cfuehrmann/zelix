use std::{
    io::{self, Read},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
};

use clap::Parser;
use regex::Regex;
use tracing::{error, info};

use crate::{get_config, init_tracing, ProjectDir};

#[derive(Parser)]
pub struct RunXunitArgs {
    #[clap(flatten)]
    project_dir: ProjectDir,

    /// The full path of the file that contains the tests
    #[clap(value_parser)]
    full_path_of_test_file: String,

    /// When test namespaces are relative to another directory than
    /// <PROJECT_DIR>, that other directory can be specified here
    #[clap(short, long)]
    test_dir: Option<String>,
}

pub fn run_xunit(
    RunXunitArgs {
        project_dir: ProjectDir { project_dir },
        full_path_of_test_file,
        test_dir,
    }: &RunXunitArgs,
) -> Result<(), ExitCode> {
    let _guards = init_tracing(project_dir)?;

    info!(
        "Project dir is {}, file is {}, test dir is {:?}",
        project_dir, full_path_of_test_file, test_dir
    );

    let session = get_config(project_dir)?.session;

    let test_dir = test_dir.as_ref().unwrap_or(project_dir);

    let which_dotnet = which::which("dotnet");
    let dotnet_path = get_path_string(&which_dotnet)?;

    let filter = get_filter(full_path_of_test_file, test_dir)?;

    run_test_in_zellij(&session, test_dir, dotnet_path, &filter)
}

fn get_path_string(witch_result: &Result<PathBuf, which::Error>) -> Result<&str, ExitCode> {
    let dotnet_path = witch_result.as_ref().map_err(|e| {
        error!("Failed to find dotnet executable: {}", e);
        ExitCode::FAILURE
    })?;

    dotnet_path.as_path().to_str().ok_or_else(|| {
        error!("Failed to convert dotnet path to string: {:?}", dotnet_path);
        ExitCode::FAILURE
    })
}

fn get_filter(full_path_of_test_file: &str, test_dir: &str) -> Result<String, ExitCode> {
    let namespace = get_namespace(full_path_of_test_file, test_dir)?;

    let mut input = String::new();
    let method_names = get_method_names(&mut input)?;

    let filter = get_qualified_method_names(&namespace, method_names).join("|");
    let filter = format!("\"{}\"", filter);

    Ok(filter)
}

fn run_test_in_zellij(
    session: &str,
    test_dir: &str,
    dotnet_path: &str,
    quoted_filter: &str,
) -> Result<(), ExitCode> {
    let mut command = Command::new("zellij");

    let command = command.args([
        "--session",
        session,
        "run",
        "--cwd",
        test_dir,
        "--direction",
        "down",
        "--",
        dotnet_path,
        "test",
        "--filter",
        quoted_filter,
    ]);

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

fn get_namespace(full_path_of_test_file: &str, test_dir: &str) -> Result<String, ExitCode> {
    let namespace_plus_dot_cs = Path::new(full_path_of_test_file)
        .strip_prefix(test_dir)
        .map_err(|e| {
            error!(
                "Failed to strip prefix '{}' from '{}': {}",
                test_dir, full_path_of_test_file, e
            );
            ExitCode::FAILURE
        })?
        .components()
        .filter_map(|component| match component {
            Component::Normal(os_str) => os_str.to_str().or_else(|| {
                error!(
                    "Failed to convert OsStr '{:?}' of path '{}' to string (where '{}' is the test dir)", 
                    os_str, full_path_of_test_file,test_dir);
                None
            }),
            c => {
                error!(
                    "Failed to convert component '{:?}' of path '{}' to string (where '{}' is the test dir)", 
                    c, full_path_of_test_file, test_dir);
                None
            }
        })
        .collect::<Vec<_>>()
        .join(".");

    let namespace = namespace_plus_dot_cs[..namespace_plus_dot_cs.len() - 3].to_string();

    Ok(namespace)
}

fn get_method_names(input: &mut String) -> Result<Vec<&str>, ExitCode> {
    io::stdin().read_to_string(input).map_err(|e| {
        error!("Failed to read from stdin: {}", e);
        ExitCode::FAILURE
    })?;

    let regex =
        Regex::new(r"\[(?:Fact|Theory)\][\s\S]*?public\s+(?:void|(?:async\s+)?Task)\s+(\w+)\(")
            .unwrap();

    let method_names: Vec<&str> = regex
        .captures_iter(input)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str())
        .collect();

    if method_names.is_empty() {
        error!("Found no tests in the input: {}", input);
        Err(ExitCode::FAILURE)
    } else {
        Ok(method_names)
    }
}

fn get_qualified_method_names(namespace: &str, method_names: Vec<&str>) -> Vec<String> {
    let mut qualified_method_names = Vec::new();

    for method_name in method_names.iter() {
        qualified_method_names.push(format!("{}.{}", namespace, method_name));
    }

    qualified_method_names
}
