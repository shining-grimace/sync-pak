use crate::provider_capabilities::{BucketAccessChecker, BucketLister, ProviderError};

/// Non-secret provider information confirmed by read-only requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderVerification {
    pub buckets: Vec<String>,
}

/// Verifies provider access and, when configured, confirms the selected bucket is visible.
pub async fn verify_provider<P: BucketAccessChecker + BucketLister>(
    provider: &P,
    configured_bucket: Option<&str>,
) -> Result<ProviderVerification, ProviderError> {
    let buckets = match provider.list_buckets().await {
        Ok(buckets) => buckets,
        Err(ProviderError::PermissionDenied) => {
            let bucket = configured_bucket.ok_or(ProviderError::PermissionDenied)?;
            provider.check_bucket_access(bucket).await?;
            return Ok(ProviderVerification {
                buckets: vec![bucket.to_owned()],
            });
        }
        Err(error) => return Err(error),
    };
    if configured_bucket.is_some_and(|bucket| !buckets.iter().any(|item| item == bucket)) {
        return Err(ProviderError::NotFound);
    }
    Ok(ProviderVerification { buckets })
}

#[cfg(test)]
mod tests {
    use std::{future::Future, task::Poll};

    use crate::provider_capabilities::{
        BucketAccessChecker, BucketLister, ProviderError, ProviderResult,
    };

    use super::{ProviderVerification, verify_provider};

    struct Provider {
        buckets: ProviderResult<Vec<String>>,
        bucket_access: ProviderResult<()>,
    }

    impl BucketAccessChecker for Provider {
        async fn check_bucket_access(&self, _: &str) -> ProviderResult<()> {
            self.bucket_access
        }
    }

    impl BucketLister for Provider {
        async fn list_buckets(&self) -> ProviderResult<Vec<String>> {
            self.buckets.clone()
        }
    }

    #[test]
    fn keeps_visible_buckets_and_requires_a_configured_bucket() {
        let provider = Provider {
            buckets: Ok(vec!["photos".into(), "backups".into()]),
            bucket_access: Ok(()),
        };

        assert_eq!(
            block_on(verify_provider(&provider, Some("backups"))),
            Ok(ProviderVerification {
                buckets: vec!["photos".into(), "backups".into()],
            })
        );
        assert_eq!(
            block_on(verify_provider(&provider, Some("missing"))),
            Err(ProviderError::NotFound)
        );
    }

    #[test]
    fn verifies_a_configured_bucket_when_bucket_listing_is_denied() {
        let provider = Provider {
            buckets: Err(ProviderError::PermissionDenied),
            bucket_access: Ok(()),
        };

        assert_eq!(
            block_on(verify_provider(&provider, Some("scoped-bucket"))),
            Ok(ProviderVerification {
                buckets: vec!["scoped-bucket".into()],
            })
        );
    }

    #[test]
    fn preserves_bucket_access_and_missing_default_errors() {
        let inaccessible = Provider {
            buckets: Err(ProviderError::PermissionDenied),
            bucket_access: Err(ProviderError::NotFound),
        };
        assert_eq!(
            block_on(verify_provider(&inaccessible, Some("missing"))),
            Err(ProviderError::NotFound)
        );
        assert_eq!(
            block_on(verify_provider(&inaccessible, None)),
            Err(ProviderError::PermissionDenied)
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
