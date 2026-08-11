use crate::configuration::{ConfigStore, ConfigurationError};

pub(crate) fn complete_welcome(configuration: &ConfigStore) -> Result<(), ConfigurationError> {
    let mut config = configuration.load()?;
    if !config.welcome_completed {
        config.welcome_completed = true;
        configuration.save(&config)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::complete_welcome;
    use crate::configuration::ConfigStore;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn completion_persists() {
        let path = std::env::temp_dir()
            .join(format!(
                "sync-pak-onboarding-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("config.json");
        let store = ConfigStore::at(path);
        complete_welcome(&store).unwrap();
        assert!(store.load().unwrap().welcome_completed);
    }
}
