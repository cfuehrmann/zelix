use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use merge::Merge;
use serde::Deserialize;
use tracing::{error, info};

#[derive(Debug, Deserialize, Merge, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub session: Option<String>,
    pub open: OpenSection,
    pub find: FindSection,
}

#[derive(Debug, Deserialize, Merge, Default)]
#[serde(rename_all = "kebab-case")]
pub struct OpenSection {
    pub terminal: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Merge, Default)]
#[serde(rename_all = "kebab-case")]
pub struct FindSection {
    pub hidden_method: Option<HiddenMethod>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum HiddenMethod {
    Open,
    Picker,
}

impl Config {
    pub fn load(project_dir: &str) -> Result<Config, ExitCode> {
        let local_path = PathBuf::from(project_dir)
            .join("zelix-config")
            .join("config.toml");

        let mut local_config = Config::load_from_file(&local_path)?;

        let home = env::var("HOME").map_err(|e| {
            error!("Failed to read $HOME: {}", e);
            ExitCode::FAILURE
        })?;

        info!("$HOME is {}", home);

        let home_path = PathBuf::from(home)
            .join(".config")
            .join("zelix")
            .join("config.toml");

        let home_config = Config::load_from_file(&home_path)?;

        local_config.merge(home_config);

        Ok(local_config)
    }

    fn load_from_file(path: &Path) -> Result<Config, ExitCode> {
        fs::read_to_string(path).map_or_else(
            |e| {
                info!(
                    "File does not exist or could not be read '{:?}': {}",
                    path, e
                );
                Ok(Config::default())
            },
            |content| {
                let config = toml::from_str(&content).map_err(|e| {
                    error!("Failed to parse the file '{:?}': {}", path, e);
                    ExitCode::FAILURE
                })?;
                info!("Configuration loaded from '{:?}'", path);
                Ok(config)
            },
        )
    }
}
