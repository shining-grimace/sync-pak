use crate::providers::capabilities::{BucketAccessChecker, BucketLister, ProviderError};

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
pub(crate) mod failure;
#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
pub(crate) mod network_access;
#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
pub(crate) mod panic;

/// Non-secret provider information confirmed by read-only requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderVerification {
    /// Discovered names are not proof that these credentials can access objects.
    pub buckets: Vec<String>,
    /// Bucket whose access was checked directly, independently of discovery.
    pub checked_bucket: Option<String>,
}

impl ProviderVerification {
    pub(crate) fn summary(&self) -> String {
        match self.checked_bucket {
            Some(_) => format!(
                "Fixed bucket access checked. {} bucket names discovered. Connection verification checks the folder.",
                self.buckets.len()
            ),
            None => format!(
                "{} bucket names discovered. Verify a connection to check bucket and folder access.",
                self.buckets.len()
            ),
        }
    }
}

/// Discovers bucket names and separately checks access to a configured bucket.
pub async fn verify_provider<P: BucketAccessChecker + BucketLister>(
    provider: &P,
    configured_bucket: Option<&str>,
) -> Result<ProviderVerification, ProviderError> {
    // Listing may include buckets this key cannot access, or omit accessible
    // cross-account buckets. Test the supplied bucket directly in either case.
    if let Some(bucket) = configured_bucket {
        provider.check_bucket_access(bucket).await?;
    }
    let buckets = match provider.list_buckets().await {
        Ok(buckets) => buckets,
        Err(ProviderError::PermissionDenied) => {
            let bucket = configured_bucket.ok_or(ProviderError::PermissionDenied)?;
            return Ok(ProviderVerification {
                buckets: vec![bucket.to_owned()],
                checked_bucket: Some(bucket.to_owned()),
            });
        }
        Err(error) => return Err(error),
    };
    Ok(ProviderVerification {
        buckets,
        checked_bucket: configured_bucket.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use std::{future::Future, task::Poll};

    use crate::providers::capabilities::{
        BucketAccessChecker, BucketLister, ProviderError, ProviderResult,
    };

    use super::{ProviderVerification, verify_provider};

    struct Provider {
        buckets: ProviderResult<Vec<String>>,
        bucket_access: ProviderResult<()>,
    }

    impl BucketAccessChecker for Provider {
        async fn check_bucket_access(&self, _: &str) -> ProviderResult<()> {
            self.bucket_access.clone()
        }
    }

    impl BucketLister for Provider {
        async fn list_buckets(&self) -> ProviderResult<Vec<String>> {
            self.buckets.clone()
        }
    }

    #[test]
    fn lists_visible_names_without_treating_them_as_access_permissions() {
        let provider = Provider {
            buckets: Ok(vec!["photos".into(), "backups".into()]),
            bucket_access: Ok(()),
        };

        assert_eq!(
            block_on(verify_provider(&provider, Some("backups"))),
            Ok(ProviderVerification {
                buckets: vec!["photos".into(), "backups".into()],
                checked_bucket: Some("backups".into()),
            })
        );
        assert!(
            block_on(verify_provider(
                &provider,
                Some("accessible-but-not-listed")
            ))
            .is_ok()
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
                checked_bucket: Some("scoped-bucket".into()),
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

    #[test]
    fn a_listed_bucket_can_still_be_inaccessible() {
        let provider = Provider {
            buckets: Ok(vec!["photos".into()]),
            bucket_access: Err(ProviderError::PermissionDenied),
        };
        assert_eq!(
            block_on(verify_provider(&provider, Some("photos"))),
            Err(ProviderError::PermissionDenied)
        );
        let discovery = block_on(verify_provider(&provider, None)).unwrap();
        assert!(discovery.checked_bucket.is_none());
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
