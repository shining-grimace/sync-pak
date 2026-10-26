use crate::{
    operations::retry::{NoopRetryObserver, RetryContext, RetryObserver, RetrySleeper},
    operations::transfer::upload::UploadError,
    providers::capabilities::{ObjectWriteMetadata, ObjectWriter},
};

pub async fn upload_contents_with_retry_and_cancellation<T: ObjectWriter, S: RetrySleeper>(
    provider: &T,
    bucket: &str,
    key: &str,
    contents: &[u8],
    write_metadata: &ObjectWriteMetadata,
    retry: &RetryContext<'_, S>,
) -> Result<(), UploadError> {
    upload_contents_with_retry_and_cancellation_and_observer(
        provider,
        bucket,
        key,
        contents,
        write_metadata,
        retry,
        &NoopRetryObserver,
    )
    .await
}

pub(crate) async fn upload_contents_with_retry_and_cancellation_and_observer<
    T: ObjectWriter,
    S: RetrySleeper,
    O: RetryObserver,
>(
    provider: &T,
    bucket: &str,
    key: &str,
    contents: &[u8],
    write_metadata: &ObjectWriteMetadata,
    retry: &RetryContext<'_, S>,
    observer: &O,
) -> Result<(), UploadError> {
    let RetryContext {
        policy,
        sleeper,
        jitter_seed,
        cancellation,
    } = *retry;

    let mut completed_attempts = 0;
    loop {
        cancellation.check().map_err(|_| UploadError::Cancelled)?;
        completed_attempts += 1;
        match provider
            .write_with_metadata(bucket, key, contents, write_metadata)
            .await
        {
            Ok(()) => return Ok(()),
            Err(error) => {
                match policy.delay_after_failure(completed_attempts, &error, None, jitter_seed) {
                    Some(retry) => {
                        observer.on_retry(retry);
                        sleeper.sleep(retry.delay).await;
                        cancellation.check().map_err(|_| UploadError::Cancelled)?;
                    }
                    None => return Err(UploadError::Provider(error)),
                }
            }
        }
    }
}
