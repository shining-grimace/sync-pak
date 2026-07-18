use std::time::Duration;

use super::RetryPolicy;
use crate::provider_capabilities::ProviderError;

#[test]
fn retries_unavailable_requests_at_most_four_times_in_total() {
    let policy = RetryPolicy::default();

    assert_eq!(
        policy
            .delay_after_failure(1, ProviderError::Unavailable, None, 0)
            .unwrap()
            .next_attempt,
        2
    );
    assert!(
        policy
            .delay_after_failure(3, ProviderError::Unavailable, None, 0)
            .is_some()
    );
    assert!(
        policy
            .delay_after_failure(4, ProviderError::Unavailable, None, 0)
            .is_none()
    );
}

#[test]
fn never_retries_deterministic_or_credential_failures() {
    let policy = RetryPolicy::default();
    for error in [
        ProviderError::Authentication,
        ProviderError::InvalidRequest,
        ProviderError::PermissionDenied,
        ProviderError::NotFound,
    ] {
        assert!(policy.delay_after_failure(1, error, None, 0).is_none());
    }
}

#[test]
fn honors_provider_delays_and_varies_the_local_backoff() {
    let policy = RetryPolicy::default();
    assert_eq!(
        policy
            .delay_after_failure(
                1,
                ProviderError::Unavailable,
                Some(Duration::from_secs(9)),
                0
            )
            .unwrap()
            .delay,
        Duration::from_secs(9)
    );
    assert_ne!(
        policy
            .delay_after_failure(2, ProviderError::Unavailable, None, 1)
            .unwrap()
            .delay,
        policy
            .delay_after_failure(2, ProviderError::Unavailable, None, 2)
            .unwrap()
            .delay
    );
}
