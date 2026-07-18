use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use crate::atomic_write::atomic_write;

use super::{AppConfig, ValidationErrors};

const CONFIG_FILE: &str = "config.json";

#[derive(Debug)]
pub enum ConfigurationError {
    Invalid(ValidationErrors),
    Io(io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(error) => write!(formatter, "The configuration is invalid: {error}"),
            Self::Io(error) => write!(
                formatter,
                "The configuration could not be accessed: {error}"
            ),
            Self::Parse(_) => formatter.write_str("The configuration file is not valid JSON."),
        }
    }
}

impl std::error::Error for ConfigurationError {}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn for_current_platform() -> Result<Self, ConfigurationError> {
        let base = config_directory().ok_or_else(|| {
            ConfigurationError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "configuration directory unavailable",
            ))
        })?;
        Ok(Self::at(base.join("sync-pak").join(CONFIG_FILE)))
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<AppConfig, ConfigurationError> {
        match fs::read(&self.path) {
            Ok(contents) => self.decode(&contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(AppConfig::default()),
            Err(error) => Err(ConfigurationError::Io(error)),
        }
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigurationError> {
        config.validate().map_err(ConfigurationError::Invalid)?;
        let contents = serde_json::to_vec_pretty(config).map_err(ConfigurationError::Parse)?;
        atomic_write(&self.path, &contents).map_err(ConfigurationError::Io)
    }

    fn decode(&self, contents: &[u8]) -> Result<AppConfig, ConfigurationError> {
        let config: AppConfig =
            serde_json::from_slice(contents).map_err(ConfigurationError::Parse)?;
        config.validate().map_err(ConfigurationError::Invalid)?;
        Ok(config)
    }
}

fn config_directory() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "android")]
    {
        env::var_os("ANDROID_DATA")
            .map(|root| PathBuf::from(root).join("data/com.shininggrimace.syncpak/files"))
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
    {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
