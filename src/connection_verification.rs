use std::{fs, io};

use crate::{
    provider_capabilities::{ObjectPrefixChecker, ProviderError},
    remote_inventory::normalize_prefix,
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
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(ConnectionVerificationError::LocalPathNotDirectory),
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
        Ok(false) => Err(ConnectionVerificationError::RemoteFolderMissing),
        Err(error) => Err(ConnectionVerificationError::Provider(error)),
    }
}

#[cfg(test)]
mod tests {
    use std::{future::Future, task::Poll};

    use crate::provider_capabilities::{ObjectPrefixChecker, ProviderResult};

    use super::{ConnectionVerificationError, verify_local_folder, verify_remote_folder};

    struct Prefix(bool);

    impl ObjectPrefixChecker for Prefix {
        async fn prefix_exists(&self, _: &str, _: &str) -> ProviderResult<bool> {
            Ok(self.0)
        }
    }

    #[test]
    fn local_verification_distinguishes_missing_and_non_directory_paths() {
        let root = std::env::temp_dir().join(format!(
            "sync-pak-connection-verification-{}",
            std::process::id()
        ));
        let missing = root.join("missing");
        assert_eq!(
            verify_local_folder(missing.to_str().unwrap()),
            Err(ConnectionVerificationError::LocalFolderMissing)
        );
        assert_eq!(
            verify_local_folder(std::env::temp_dir().to_str().unwrap()),
            Ok(())
        );
        assert_eq!(
            verify_local_folder(std::env::current_exe().unwrap().to_str().unwrap()),
            Err(ConnectionVerificationError::LocalPathNotDirectory)
        );
    }

    #[test]
    fn remote_verification_reports_a_missing_cloud_path() {
        assert_eq!(
            block_on(verify_remote_folder(&Prefix(false), "bucket", "photos")),
            Err(ConnectionVerificationError::RemoteFolderMissing)
        );
        assert_eq!(
            block_on(verify_remote_folder(&Prefix(false), "bucket", "")),
            Err(ConnectionVerificationError::RemoteFolderMissing)
        );
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test checker should resolve immediately"),
        }
    }
}
