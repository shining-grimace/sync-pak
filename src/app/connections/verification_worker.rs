use std::{borrow::Cow, path::PathBuf};

use crate::{
    app::connections::verification::{ConnectionVerificationError, verify_local_folder},
    configuration::{ConfigStore, ConnectionDraft},
    providers::capabilities::ProviderError,
    providers::errors::connectivity_failure::ProviderConnectivityFailure,
};

#[cfg(feature = "provider-s3")]
use crate::{
    app::connections::verification::verify_remote_folder, configuration::ProviderRepository,
    platform::PlatformCredentialStore,
};

pub(crate) fn verify(
    configuration_path: PathBuf,
    connection: ConnectionDraft,
) -> Result<(), VerificationFailure> {
    verify_local_folder(&connection.local_path).map_err(VerificationFailure::from)?;
    verify_remote(configuration_path, connection)
}

pub(crate) fn verify_saved(
    configuration_path: PathBuf,
    connection_id: String,
) -> Result<(), VerificationFailure> {
    let connection = ConfigStore::at(configuration_path.clone())
        .load()
        .map_err(|_| VerificationFailure::Unexpected)?
        .connections
        .into_iter()
        .find(|connection| connection.id.as_str() == connection_id)
        .ok_or(VerificationFailure::Unexpected)?;
    verify(
        configuration_path,
        ConnectionDraft {
            name: connection.name,
            provider_id: connection.provider_id,
            bucket: connection.bucket,
            remote_path: connection.remote_path,
            local_path: connection.local_path,
            mode: connection.mode,
            keep_last_archives: connection.keep_last_archives,
            verified: false,
        },
    )
}

#[cfg(feature = "provider-s3")]
fn verify_remote(
    configuration_path: PathBuf,
    connection: ConnectionDraft,
) -> Result<(), VerificationFailure> {
    let configuration = ConfigStore::at(configuration_path);
    let provider = configuration
        .load()
        .map_err(|_| VerificationFailure::Unexpected)?
        .providers
        .into_iter()
        .find(|provider| provider.id == connection.provider_id)
        .ok_or(VerificationFailure::ProviderMissing)?;
    let store = PlatformCredentialStore::new().map_err(|_| VerificationFailure::Credentials)?;
    let credentials = ProviderRepository::new(&configuration, &store)
        .load_credentials(&provider.id)
        .map_err(|_| VerificationFailure::Credentials)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| VerificationFailure::Unexpected)?;
    let transport = runtime
        .block_on(crate::providers::s3::transport::S3Transport::connect(
            &provider,
            credentials,
        ))
        .map_err(VerificationFailure::from)?;
    runtime
        .block_on(verify_remote_folder(
            &transport,
            &connection.bucket,
            &connection.remote_path,
        ))
        .map_err(VerificationFailure::from)
}

#[cfg(not(feature = "provider-s3"))]
fn verify_remote(_: PathBuf, _: ConnectionDraft) -> Result<(), VerificationFailure> {
    Err(VerificationFailure::Unavailable)
}

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VerificationFailure {
    Credentials,
    LocalFolderMissing,
    LocalPathNotDirectory,
    LocalFolderUnavailable,
    RemoteFolderMissing,
    InvalidRemotePath,
    ProviderMissing,
    Authentication,
    Connectivity(ProviderConnectivityFailure),
    PermissionDenied,
    Unavailable,
    Unexpected,
}

impl From<ConnectionVerificationError> for VerificationFailure {
    fn from(error: ConnectionVerificationError) -> Self {
        match error {
            ConnectionVerificationError::LocalFolderMissing => Self::LocalFolderMissing,
            ConnectionVerificationError::LocalPathNotDirectory => Self::LocalPathNotDirectory,
            ConnectionVerificationError::LocalFolderUnavailable => Self::LocalFolderUnavailable,
            ConnectionVerificationError::RemoteFolderMissing => Self::RemoteFolderMissing,
            ConnectionVerificationError::InvalidRemotePath => Self::InvalidRemotePath,
            ConnectionVerificationError::Provider(error) => Self::from(error),
        }
    }
}

impl From<ProviderError> for VerificationFailure {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::Authentication => Self::Authentication,
            ProviderError::Certificate(error) => Self::Connectivity(error.into()),
            ProviderError::NotFound => Self::RemoteFolderMissing,
            ProviderError::PermissionDenied => Self::PermissionDenied,
            ProviderError::Transport(error) => Self::Connectivity(error.into()),
            ProviderError::Unavailable => Self::Unavailable,
            _ => Self::Unexpected,
        }
    }
}

impl VerificationFailure {
    pub(crate) fn diagnostic(&self) -> Cow<'static, str> {
        match self {
            Self::Credentials => "saved credential access failed",
            Self::LocalFolderMissing => "local folder does not exist",
            Self::LocalPathNotDirectory => "local path is not a directory",
            Self::LocalFolderUnavailable => "local folder could not be accessed",
            Self::RemoteFolderMissing => "cloud bucket or remote folder does not exist",
            Self::InvalidRemotePath => "remote folder path is invalid",
            Self::ProviderMissing => "connection provider is missing",
            Self::Authentication => "provider rejected saved credentials",
            Self::Connectivity(failure) => return failure.diagnostic(),
            Self::PermissionDenied => "provider denied access to the configured cloud path",
            Self::Unavailable => "provider reported that its service is unavailable",
            Self::Unexpected => "connection verification failed",
        }
        .into()
    }

    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::Credentials => {
                "SyncPak could not access the saved provider credentials. Unlock protected storage, then try again."
            }
            Self::LocalFolderMissing => {
                "The configured local folder does not exist. Choose an existing folder, then try again."
            }
            Self::LocalPathNotDirectory => {
                "The configured local path is not a folder. Choose a folder, then try again."
            }
            Self::LocalFolderUnavailable => {
                "SyncPak could not access the configured local folder. Check its permissions, then try again."
            }
            Self::RemoteFolderMissing => {
                "The configured cloud bucket or remote folder does not exist. Check the cloud path, then try again."
            }
            Self::InvalidRemotePath => {
                "The configured remote folder path is invalid. Edit it, then try again."
            }
            Self::ProviderMissing => {
                "This connection's provider no longer exists. Choose another provider."
            }
            Self::Authentication => {
                "The provider rejected its saved credentials. Update and verify the provider, then try again."
            }
            Self::Connectivity(failure) => failure.message(),
            Self::PermissionDenied => {
                "The provider credentials cannot access the configured cloud bucket or remote folder."
            }
            Self::Unavailable => {
                "The provider responded that its service is temporarily unavailable. The endpoint was reached; this does not indicate invalid connection settings."
            }
            Self::Unexpected => {
                "SyncPak could not verify this connection. Check its settings and try again."
            }
        }
    }
}

#[cfg(test)]
mod tests;
