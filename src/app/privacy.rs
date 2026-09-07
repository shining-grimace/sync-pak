use std::path::PathBuf;

use slint::ComponentHandle;

use crate::{AppWindow, configuration::ConfigStore};

pub(crate) fn configure(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_open_settings_directory(move || {
        let Some(window) = weak.upgrade() else { return };
        refresh(&window);
        let path = window.get_settings_directory();
        if path.is_empty() {
            return;
        }
        let weak = window.as_weak();
        std::thread::spawn(move || {
            if open_directory(PathBuf::from(path.as_str())).is_err() {
                let _ = weak.upgrade_in_event_loop(|window| {
                    window.set_status_message(
                        "The settings directory could not be opened in your file browser.".into(),
                    );
                });
            }
        });
    });
}

pub(crate) fn refresh(window: &AppWindow) {
    let directory = ConfigStore::for_current_platform()
        .ok()
        .and_then(|store| stored_directory(&store));
    window.set_settings_directory(
        directory
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
            .into(),
    );
}

fn stored_directory(store: &ConfigStore) -> Option<PathBuf> {
    if !store.path().is_file() {
        return None;
    }
    std::path::absolute(store.path().parent()?).ok()
}

fn open_directory(path: PathBuf) -> std::io::Result<()> {
    #[cfg(not(target_os = "android"))]
    {
        #[cfg(target_os = "windows")]
        let opener = "explorer.exe";
        #[cfg(not(target_os = "windows"))]
        let opener = "xdg-open";
        let status = std::process::Command::new(opener).arg(path).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other("file browser failed to open"))
        }
    }
    #[cfg(target_os = "android")]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Android does not expose private settings to file browsers",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::stored_directory;
    use crate::configuration::{AppConfig, ConfigStore};

    #[test]
    fn local_data_requires_a_stored_settings_file() {
        let directory =
            std::env::temp_dir().join(format!("sync-pak-privacy-{}", uuid::Uuid::new_v4()));
        let store = ConfigStore::at(directory.join("config.json"));
        assert_eq!(stored_directory(&store), None);
        std::fs::create_dir_all(&directory).unwrap();
        assert_eq!(stored_directory(&store), None);
        store.save(&AppConfig::default()).unwrap();
        assert_eq!(stored_directory(&store), Some(directory.clone()));
        std::fs::remove_file(store.path()).unwrap();
        assert_eq!(stored_directory(&store), None);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
