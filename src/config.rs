use std::fmt::Display;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub(crate) enum ConfigError {
    NoBaseUrl,
    NoApiToken,
    WriteFailute,
    LoadError,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoBaseUrl => write!(f, "No base url was found"),
            ConfigError::NoApiToken => write!(f, "No api token was found"),
            ConfigError::WriteFailute => write!(f, "Failure writing the config file"),
            ConfigError::LoadError => write!(f, "Error loading the config file"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub token: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Area {
    pub id: String,
    pub entities: Vec<String>,
}
