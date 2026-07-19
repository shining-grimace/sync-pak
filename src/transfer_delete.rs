use std::{error::Error, fmt};

use crate::{
    cancellation::CancellationToken,
    inventory::RelativePath,
    provider_capabilities::{ObjectDeleter, ProviderError},
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
};

/// Removes one local file, symlink, or already-empty directory beneath a transfer root.
pub fn delete_local(
    root: &LocalTransferRoot,
    relative: &RelativePath,
    cancellation: &CancellationToken,
) -> Result<(), TransferDeleteError> {
    cancellation
        .check()
        .map_err(|_| TransferDeleteError::Cancelled)?;
    let path = root.resolve(relative);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(TransferDeleteError::Local(error)),
    };
    let result = if metadata.file_type().is_dir() {
        std::fs::remove_dir(path)
    } else {
        std::fs::remove_file(path)
    };
    result.map_err(TransferDeleteError::Local)
}

/// Removes one provider object beneath a transfer prefix.
pub async fn delete_remote<T: ObjectDeleter>(
    provider: &T,
    bucket: &str,
    prefix: &RemoteTransferPrefix,
    relative: &RelativePath,
    cancellation: &CancellationToken,
) -> Result<(), TransferDeleteError> {
    cancellation
        .check()
        .map_err(|_| TransferDeleteError::Cancelled)?;
    provider
        .delete(bucket, &prefix.resolve(relative))
        .await
        .map_err(TransferDeleteError::Provider)
}

#[derive(Debug)]
pub enum TransferDeleteError {
    Cancelled,
    Local(std::io::Error),
    Provider(ProviderError),
}

impl fmt::Display for TransferDeleteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("deletion was cancelled"),
            Self::Local(error) => {
                write!(formatter, "could not remove the local destination: {error}")
            }
            Self::Provider(error) => error.fmt(formatter),
        }
    }
}

impl Error for TransferDeleteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cancelled => None,
            Self::Local(error) => Some(error),
            Self::Provider(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::Mutex,
        task::{Context, Poll, Waker},
    };

    use crate::{
        cancellation::CancellationToken,
        inventory::RelativePath,
        provider_capabilities::{ObjectDeleter, ProviderResult},
        transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
    };

    use super::{delete_local, delete_remote};

    #[derive(Default)]
    struct Provider(Mutex<Vec<String>>);

    impl ObjectDeleter for Provider {
        async fn delete(&self, _: &str, key: &str) -> ProviderResult<()> {
            self.0.lock().unwrap().push(key.into());
            Ok(())
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test provider must not suspend"),
        }
    }

    #[test]
    fn removes_local_files_and_empty_directories() {
        let root = std::env::temp_dir().join(format!("sync-pak-delete-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("file"), "contents").unwrap();
        std::fs::create_dir(root.join("empty")).unwrap();
        let root = LocalTransferRoot::new(&root);

        delete_local(
            &root,
            &RelativePath::new("file").unwrap(),
            &CancellationToken::default(),
        )
        .unwrap();
        delete_local(
            &root,
            &RelativePath::new("empty").unwrap(),
            &CancellationToken::default(),
        )
        .unwrap();

        assert!(!root.resolve(&RelativePath::new("file").unwrap()).exists());
        assert!(!root.resolve(&RelativePath::new("empty").unwrap()).exists());
        std::fs::remove_dir(root.as_path()).unwrap();
    }

    #[test]
    fn removes_prefixed_remote_objects() {
        let provider = Provider::default();

        block_on(delete_remote(
            &provider,
            "bucket",
            &RemoteTransferPrefix::new("sync").unwrap(),
            &RelativePath::new("file").unwrap(),
            &CancellationToken::default(),
        ))
        .unwrap();

        assert_eq!(provider.0.lock().unwrap().as_slice(), ["sync/file"]);
    }
}
