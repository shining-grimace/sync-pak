use std::borrow::Cow;

use crate::{
    app::connections::verification::ConnectionVerificationError,
    providers::capabilities::ProviderError,
    providers::errors::connectivity_failure::ProviderConnectivityFailure,
};

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VerificationFailure {
    Credentials,
    LocalFolderMissing,
    LocalPathNotDirectory,
    LocalFolderUnavailable,
    RemoteFolderMissing,
    MissingRemotePrefix,
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
            ConnectionVerificationError::RemoteFolderMissing => Self::MissingRemotePrefix,
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
            Self::MissingRemotePrefix => "remote folder does not exist",
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
            Self::MissingRemotePrefix => {
                "The configured remote folder does not exist. Check the cloud path, then try again."
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
