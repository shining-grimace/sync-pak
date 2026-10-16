//! Process-wide admission and configuration protection shared by UI and service callers.
use super::{AppConfig, ConfigurationError};
use crate::operations::request::RunRequest;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

#[derive(Default)]
pub struct RunState {
    active: Option<RunRequest>,
    editing_providers: HashSet<String>,
}
static STATES: OnceLock<Mutex<HashMap<PathBuf, RunState>>> = OnceLock::new();
fn states() -> &'static Mutex<HashMap<PathBuf, RunState>> {
    STATES.get_or_init(Default::default)
}
fn error(message: &str) -> ConfigurationError {
    ConfigurationError::Portable(message.into())
}

/// Holds admission while resolving and submitting a run, so a second caller cannot race it.
pub fn admit<T>(
    path: &Path,
    submit: impl FnOnce(&mut RunState) -> Result<T, ConfigurationError>,
) -> Result<T, ConfigurationError> {
    let mut states = states().lock().expect("run state mutex poisoned");
    let state = states.entry(path.into()).or_default();
    if state.active.is_some() {
        return Err(error(
            "An operation is already running. Wait for it to finish.",
        ));
    }
    submit(state)
}
impl RunState {
    pub fn reserve(&mut self, request: &RunRequest) -> Result<(), ConfigurationError> {
        if self
            .editing_providers
            .contains(request.provider.id.as_str())
        {
            return Err(error(
                "This provider is being updated. Try again when it finishes.",
            ));
        }
        self.active = Some(request.clone());
        Ok(())
    }
}
pub fn release(path: &Path, connection_id: &str) {
    if let Some(state) = states()
        .lock()
        .expect("run state mutex poisoned")
        .get_mut(path)
        && state
            .active
            .as_ref()
            .is_some_and(|r| r.connection.id.as_str() == connection_id)
    {
        state.active = None;
    }
}
pub fn active(path: &Path) -> Option<RunRequest> {
    states()
        .lock()
        .expect("run state mutex poisoned")
        .get(path)
        .and_then(|s| s.active.clone())
}

/// Validates and commits settings under the same lock used for run admission.
pub fn save<T>(
    path: &Path,
    config: &AppConfig,
    commit: impl FnOnce() -> Result<T, ConfigurationError>,
) -> Result<T, ConfigurationError> {
    let states = states().lock().expect("run state mutex poisoned");
    if let Some(request) = states.get(path).and_then(|s| s.active.as_ref())
        && (config
            .connections
            .iter()
            .find(|c| c.id == request.connection.id)
            != Some(&request.connection)
            || config
                .providers
                .iter()
                .find(|p| p.id == request.provider.id)
                != Some(&request.provider))
    {
        return Err(error(
            "The running connection and its provider cannot be changed or deleted until the operation finishes.",
        ));
    }
    commit()
}

/// Protects credentials throughout a provider update, including asynchronous UI saves.
pub struct ProviderEdit {
    path: PathBuf,
    id: String,
}
pub fn edit_provider(path: &Path, id: &str) -> Result<ProviderEdit, ConfigurationError> {
    let mut states = states().lock().expect("run state mutex poisoned");
    let state = states.entry(path.into()).or_default();
    if state
        .active
        .as_ref()
        .is_some_and(|r| r.provider.id.as_str() == id)
    {
        return Err(error("This provider is in use by a running operation."));
    }
    if !state.editing_providers.insert(id.into()) {
        return Err(error("This provider is already being updated."));
    }
    Ok(ProviderEdit {
        path: path.into(),
        id: id.into(),
    })
}
impl Drop for ProviderEdit {
    fn drop(&mut self) {
        if let Some(state) = states()
            .lock()
            .expect("run state mutex poisoned")
            .get_mut(&self.path)
        {
            state.editing_providers.remove(&self.id);
        }
    }
}
