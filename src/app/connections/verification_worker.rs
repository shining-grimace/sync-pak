mod failure;
pub(crate) use failure::VerificationFailure;

use std::path::PathBuf;

use crate::{
    app::connections::verification::verify_local_folder,
    configuration::{ConfigStore, ConnectionDraft},
};

#[cfg(feature = "provider-s3")]
use crate::{
    app::connections::verification::{create_remote_folder as create_folder, verify_remote_folder},
    configuration::ProviderRepository,
    platform::PlatformCredentialStore,
};

pub(crate) fn verify(
    configuration_path: PathBuf,
    connection: ConnectionDraft,
) -> Result<(), VerificationFailure> {
    check_paths(configuration_path, connection, RemoteFolderAction::Check)
}

pub(crate) fn create_remote_folder(
    configuration_path: PathBuf,
    connection: ConnectionDraft,
) -> Result<(), VerificationFailure> {
    check_paths(configuration_path, connection, RemoteFolderAction::Create)
}

enum RemoteFolderAction {
    Check,
    Create,
}

fn check_paths(
    configuration_path: PathBuf,
    connection: ConnectionDraft,
    action: RemoteFolderAction,
) -> Result<(), VerificationFailure> {
    verify_local_folder(&connection.local_path).map_err(VerificationFailure::from)?;
    check_remote(configuration_path, connection, action)
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
            transport: connection.transport,
            name: connection.name,
            provider_id: connection.provider_id,
            bucket: connection.bucket,
            remote_path: connection.remote_path,
            local_path: connection.local_path,
            mode: connection.mode,
            allow_upload: connection.allow_upload,
            allow_download: connection.allow_download,
            show_preview: connection.show_preview,
            keep_last_archives: connection.keep_last_archives,
            verified: false,
        },
    )
}

#[cfg(feature = "provider-s3")]
fn check_remote(
    configuration_path: PathBuf,
    connection: ConnectionDraft,
    action: RemoteFolderAction,
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
        .block_on(
            crate::providers::s3::transport::S3Transport::connect_bucket(
                &provider,
                credentials,
                &connection.bucket,
                connection.transport.as_ref(),
            ),
        )
        .map_err(VerificationFailure::from)?;
    match action {
        RemoteFolderAction::Check => runtime.block_on(verify_remote_folder(
            &transport,
            &connection.bucket,
            &connection.remote_path,
        )),
        RemoteFolderAction::Create => runtime.block_on(create_folder(
            &transport,
            &connection.bucket,
            &connection.remote_path,
        )),
    }
    .map_err(VerificationFailure::from)
}

#[cfg(not(feature = "provider-s3"))]
fn check_remote(
    _: PathBuf,
    _: ConnectionDraft,
    _: RemoteFolderAction,
) -> Result<(), VerificationFailure> {
    Err(VerificationFailure::Unavailable)
}

#[cfg(test)]
mod tests;
