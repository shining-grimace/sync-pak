use std::io;

use crate::{
    inventory::remote::normalize_prefix,
    operations::transfer::paths::LocalTransferRoot,
    providers::capabilities::{ObjectPrefixChecker, ProviderError},
};

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConnectionVerificationError {
    LocalFolderMissing,
    LocalPathNotDirectory,
    LocalFolderUnavailable,
    RemoteFolderMissing,
    InvalidRemotePath,
    Provider(ProviderError),
}

pub(crate) fn verify_local_folder(path: &str) -> Result<(), ConnectionVerificationError> {
    match LocalTransferRoot::from_config(path).verify() {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotADirectory => {
            Err(ConnectionVerificationError::LocalPathNotDirectory)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(ConnectionVerificationError::LocalFolderMissing)
        }
        Err(_) => Err(ConnectionVerificationError::LocalFolderUnavailable),
    }
}

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
pub(crate) async fn verify_remote_folder<C: ObjectPrefixChecker>(
    checker: &C,
    bucket: &str,
    remote_path: &str,
) -> Result<(), ConnectionVerificationError> {
    let prefix = normalize_prefix(remote_path)
        .map_err(|_| ConnectionVerificationError::InvalidRemotePath)?;
    match checker.prefix_exists(bucket, &prefix).await {
        Ok(true) => Ok(()),
        Ok(false) if prefix.is_empty() => Err(ConnectionVerificationError::Provider(
            ProviderError::NotFound,
        )),
        Ok(false) => Err(ConnectionVerificationError::RemoteFolderMissing),
        Err(error) => Err(ConnectionVerificationError::Provider(error)),
    }
}

#[cfg(any(feature = "provider-s3", test))]
pub(crate) async fn create_remote_folder<C>(
    storage: &C,
    bucket: &str,
    remote_path: &str,
) -> Result<(), ConnectionVerificationError>
where
    C: ObjectPrefixChecker + crate::providers::capabilities::ObjectPrefixCreator,
{
    let prefix = normalize_prefix(remote_path)
        .map_err(|_| ConnectionVerificationError::InvalidRemotePath)?;
    if prefix.is_empty() {
        return Err(ConnectionVerificationError::InvalidRemotePath);
    }
    if !storage
        .prefix_exists(bucket, &prefix)
        .await
        .map_err(ConnectionVerificationError::Provider)?
    {
        storage
            .create_prefix(bucket, &prefix)
            .await
            .map_err(ConnectionVerificationError::Provider)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
