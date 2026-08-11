use super::{VerificationStates, status};
use crate::app::providers::bucket_cache::{ProviderBucketCache, record, remove};

#[test]
fn distinguishes_current_previous_and_absent_verification() {
    let states: VerificationStates = Default::default();
    let buckets: ProviderBucketCache = Default::default();

    record(&buckets, "provider", Vec::new());
    assert_eq!(
        status(&states, &buckets, "provider", true),
        "Verified this session"
    );

    remove(&buckets, "provider");
    assert_eq!(
        status(&states, &buckets, "provider", true),
        "Previously verified"
    );
    assert_eq!(status(&states, &buckets, "provider", false), "Not verified");
}
