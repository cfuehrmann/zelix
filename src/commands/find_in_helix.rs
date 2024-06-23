use std::process::ExitCode;

use clap::Parser;
use tracing::warn;

use crate::ProjectDir;

use super::helpers::write_chars_and_rel_path_to_helix;

#[derive(Parser)]
pub struct FindInHelixArgs {
    #[clap(flatten)]
    project_dir: ProjectDir,

    /// The full path of the file to find
    #[clap(value_parser)]
    full_path_of_file: String,
}

pub fn find_in_helix(
    FindInHelixArgs {
        project_dir: ProjectDir { project_dir },
        full_path_of_file,
    }: &FindInHelixArgs,
) -> Result<(), ExitCode> {
    write_chars_and_rel_path_to_helix(" f", project_dir, full_path_of_file)
}

pub fn find(
    FindInHelixArgs {
        project_dir: ProjectDir { project_dir },
        full_path_of_file,
    }: &FindInHelixArgs,
) -> Result<(), ExitCode> {
    warn!("The find command is deprecated. Use find-in-helix instead");
    write_chars_and_rel_path_to_helix(" f", project_dir, full_path_of_file)
}
