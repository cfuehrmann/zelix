use std::{
    fs,
    process::{Command, ExitCode},
};

use clap::Parser;
use syn::{visit::Visit, ItemFn};
use tracing::{error, info};

use crate::{get_config, init_tracing, ProjectDir};

#[derive(Parser)]
pub struct RunCargoNextestArgs {
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

pub fn run_cargo_nextest(
    RunCargoNextestArgs {
        project_dir: ProjectDir { project_dir },
        full_path_of_test_file,
        test_dir,
    }: &RunCargoNextestArgs,
) -> Result<(), ExitCode> {
    let _guards = init_tracing(project_dir)?;

    info!(
        "Project dir is {}, file is {}, test dir is {:?}",
        project_dir, full_path_of_test_file, test_dir
    );

    let session = get_config(project_dir)?.session;
    let test_dir = test_dir.as_ref().unwrap_or(project_dir);

    let code = fs::read_to_string(full_path_of_test_file).map_err(|e| {
        error!("Failed to read the test file: {}", e);
        ExitCode::FAILURE
    })?;

    let test_functions = extract_test_functions(&code)?;

    if test_functions.is_empty() {
        error!("No tests found.");
        return Err(ExitCode::FAILURE);
    }

    let mut command = Command::new("zellij");

    let command = command.args([
        "--session",
        &session,
        "run",
        "--cwd",
        test_dir,
        "--direction",
        "down",
        "--",
        "cargo",
        "nextest",
        "run",
    ]);

    for function in test_functions {
        command.arg("-E").arg(format!("test({})", function));
    }

    info!("Running: {:?}", command);

    let output = command.output().map_err(|e| {
        error!("Failed to execute cargo nextest run: {}", e);
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

    let status = output.status;

    if status.success() {
        info!("Command '{:?}' succeeded", command);
        Ok(())
    } else {
        error!("Command {:?} failed with status: {}", command, status);
        Err(ExitCode::FAILURE)
    }
}

fn extract_test_functions(code: &str) -> Result<Vec<String>, ExitCode> {
    let syntax_tree = syn::parse_file(code).map_err(|e| {
        error!("Failed to parse test file: {}", e);
        ExitCode::FAILURE
    })?;

    let mut visitor = TestFunctionVisitor::default();
    visitor.visit_file(&syntax_tree);
    Ok(visitor.test_functions)
}

#[derive(Default)]
struct TestFunctionVisitor {
    test_functions: Vec<String>,
}

impl<'ast> Visit<'ast> for TestFunctionVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        for attr in &i.attrs {
            if attr.path().is_ident("test") {
                let func_name = i.sig.ident.to_string();
                self.test_functions.push(func_name);
            }
        }
    }
}
