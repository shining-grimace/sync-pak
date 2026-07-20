use std::{error::Error, fmt, future::Future};

use crate::{
    archive_retention::{ArchiveRecord, ArchiveRetentionError, prune_after_success},
    cancellation::CancellationToken,
};

/// Removes a previously stored archive from its local or remote destination.
pub trait ArchiveRemover {
    type Error;

    fn remove(&self, archive: &ArchiveRecord) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Prunes only records authorized by a successful new archive and its retention policy.
pub async fn prune_archives<R: ArchiveRemover>(
    remover: &R,
    existing: &[ArchiveRecord],
    new_archive: &ArchiveRecord,
    keep_last: u32,
    cancellation: &CancellationToken,
) -> Result<Vec<ArchiveRecord>, ArchivePruneError<R::Error>> {
    let candidates = prune_after_success(existing, new_archive, keep_last)
        .map_err(ArchivePruneError::Retention)?;
    let mut removed = Vec::new();
    for archive in candidates {
        if cancellation.is_cancelled() {
            return Err(ArchivePruneError::Cancelled { removed });
        }
        if let Err(error) = remover.remove(&archive).await {
            return Err(ArchivePruneError::Remove { error, removed });
        }
        removed.push(archive);
    }
    Ok(removed)
}

#[derive(Debug)]
pub enum ArchivePruneError<E> {
    Retention(ArchiveRetentionError),
    Cancelled {
        removed: Vec<ArchiveRecord>,
    },
    Remove {
        error: E,
        removed: Vec<ArchiveRecord>,
    },
}

impl<E: fmt::Display> fmt::Display for ArchivePruneError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Retention(error) => error.fmt(formatter),
            Self::Cancelled { .. } => formatter.write_str("archive pruning was cancelled"),
            Self::Remove { error, .. } => {
                write!(formatter, "could not remove an older archive: {error}")
            }
        }
    }
}

impl<E: Error + 'static> Error for ArchivePruneError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Retention(error) => Some(error),
            Self::Remove { error, .. } => Some(error),
            Self::Cancelled { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        future::Future,
        sync::Mutex,
        task::{Context, Poll, Waker},
    };

    use crate::{
        archive_retention::ArchiveRecord, cancellation::CancellationToken,
        configuration::ConnectionId,
    };

    use super::{ArchiveRemover, prune_archives};

    #[derive(Default)]
    struct Remover(Mutex<Vec<String>>);

    impl ArchiveRemover for Remover {
        type Error = Infallible;

        fn remove(&self, archive: &ArchiveRecord) -> impl Future<Output = Result<(), Self::Error>> {
            async move {
                self.0.lock().unwrap().push(archive.location.clone());
                Ok(())
            }
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test remover must not suspend"),
        }
    }

    fn archive(
        connection_id: &ConnectionId,
        location: &str,
        created_at_utc: &str,
    ) -> ArchiveRecord {
        ArchiveRecord {
            connection_id: connection_id.clone(),
            location: location.into(),
            created_at_utc: created_at_utc.into(),
        }
    }

    #[test]
    fn removes_only_owned_archives_outside_retention_after_success() {
        let connection = ConnectionId::new();
        let other = ConnectionId::new();
        let new_archive = archive(&connection, "new.zip", "20260720-120000Z");
        let remover = Remover::default();

        let removed = block_on(prune_archives(
            &remover,
            &[
                archive(&connection, "old.zip", "20260719-120000Z"),
                archive(&other, "other.zip", "20260718-120000Z"),
            ],
            &new_archive,
            1,
            &CancellationToken::default(),
        ))
        .unwrap();

        assert_eq!(
            removed,
            [archive(&connection, "old.zip", "20260719-120000Z")]
        );
        assert_eq!(remover.0.lock().unwrap().as_slice(), ["old.zip"]);
    }
}
