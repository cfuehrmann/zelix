use std::process::ExitCode;

use clap::Parser;

use crate::ProjectDir;

use super::helpers::write_chars_and_rel_path_to_helix;

#[derive(Parser)]
pub struct OpenInHelixArgs {
    #[clap(flatten)]
    project_dir: ProjectDir,

    /// The full path of the file to open
    #[clap(value_parser)]
    full_path_of_file: String,
}

pub fn open_in_helix(
    OpenInHelixArgs {
        project_dir: ProjectDir { project_dir },
        full_path_of_file,
    }: &OpenInHelixArgs,
) -> Result<(), ExitCode> {
    write_chars_and_rel_path_to_helix(":o ", project_dir, full_path_of_file)
}
