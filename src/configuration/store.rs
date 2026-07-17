use std::{
    env, fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use super::{AppConfig, CURRENT_SCHEMA_VERSION, ValidationErrors};

const CONFIG_FILE: &str = "config.json";

#[derive(Debug)]
pub enum ConfigurationError {
    Invalid(ValidationErrors),
    Io(io::Error),
    Parse(serde_json::Error),
    UnsupportedSchema(u32),
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
            Self::UnsupportedSchema(version) => write!(
                formatter,
                "Configuration schema version {version} is unsupported."
            ),
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
        let mut value: serde_json::Value =
            serde_json::from_slice(contents).map_err(ConfigurationError::Parse)?;
        migrate(&mut value)?;
        let config: AppConfig = serde_json::from_value(value).map_err(ConfigurationError::Parse)?;
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

fn migrate(value: &mut serde_json::Value) -> Result<(), ConfigurationError> {
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as u32;
    match version {
        CURRENT_SCHEMA_VERSION => Ok(()),
        0 => {
            let object = value
                .as_object_mut()
                .ok_or(ConfigurationError::UnsupportedSchema(0))?;
            object.insert("schema_version".to_owned(), CURRENT_SCHEMA_VERSION.into());
            object
                .entry("providers")
                .or_insert_with(|| serde_json::json!([]));
            object
                .entry("connections")
                .or_insert_with(|| serde_json::json!([]));
            Ok(())
        }
        other => Err(ConfigurationError::UnsupportedSchema(other)),
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "configuration path has no parent directory",
        )
    })?;
    fs::create_dir_all(directory)?;
    let temporary = directory.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    let mut temporary_file = fs::File::create(&temporary)?;
    temporary_file.write_all(contents)?;
    temporary_file.sync_all()?;
    drop(temporary_file);
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

#[cfg(test)]
mod tests {
    use super::ConfigStore;
    use crate::configuration::AppConfig;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn saves_and_loads_a_versioned_configuration() {
        let path = std::env::temp_dir()
            .join(format!(
                "sync-pak-config-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("config.json");
        let store = ConfigStore::at(path.clone());
        store.save(&AppConfig::default()).unwrap();
        assert_eq!(store.load().unwrap(), AppConfig::default());
        assert!(fs::read_to_string(path).unwrap().contains("schema_version"));
    }

    #[test]
    fn migrates_an_unversioned_empty_configuration() {
        let path = std::env::temp_dir()
            .join(format!(
                "sync-pak-migration-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("config.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{}").unwrap();

        assert_eq!(ConfigStore::at(path).load().unwrap(), AppConfig::default());
    }
}
