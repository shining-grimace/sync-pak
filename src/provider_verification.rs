use crate::provider_capabilities::{BucketLister, ProviderError};

/// Non-secret provider information confirmed by a read-only bucket-list request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderVerification {
    pub buckets: Vec<String>,
}

/// Verifies provider access and, when configured, confirms the selected bucket is visible.
pub async fn verify_provider<L: BucketLister>(
    provider: &L,
    configured_bucket: Option<&str>,
) -> Result<ProviderVerification, ProviderError> {
    let buckets = provider.list_buckets().await?;
    if configured_bucket.is_some_and(|bucket| !buckets.iter().any(|item| item == bucket)) {
        return Err(ProviderError::NotFound);
    }
    Ok(ProviderVerification { buckets })
}

#[cfg(test)]
mod tests {
    use std::{future::Future, task::Poll};

    use crate::provider_capabilities::{BucketLister, ProviderError, ProviderResult};

    use super::{ProviderVerification, verify_provider};

    struct Buckets(ProviderResult<Vec<String>>);

    impl BucketLister for Buckets {
        async fn list_buckets(&self) -> ProviderResult<Vec<String>> {
            self.0.clone()
        }
    }

    #[test]
    fn keeps_visible_buckets_and_requires_a_configured_bucket() {
        let buckets = Buckets(Ok(vec!["photos".into(), "backups".into()]));

        assert_eq!(
            block_on(verify_provider(&buckets, Some("backups"))),
            Ok(ProviderVerification {
                buckets: vec!["photos".into(), "backups".into()],
            })
        );
        assert_eq!(
            block_on(verify_provider(&buckets, Some("missing"))),
            Err(ProviderError::NotFound)
        );
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test provider should resolve immediately"),
        }
    }
}
