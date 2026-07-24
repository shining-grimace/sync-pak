use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Launch-scoped, non-secret bucket listings obtained from successful verification.
pub(crate) type ProviderBucketCache = Rc<RefCell<HashMap<String, Vec<String>>>>;

pub(crate) fn record(cache: &ProviderBucketCache, provider_id: &str, buckets: Vec<String>) {
    cache.borrow_mut().insert(provider_id.into(), buckets);
}

pub(crate) fn buckets(cache: &ProviderBucketCache, provider_id: &str) -> Option<Vec<String>> {
    cache.borrow().get(provider_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::{ProviderBucketCache, buckets, record};

    #[test]
    fn preserves_an_empty_verified_listing_separately_from_no_listing() {
        let cache: ProviderBucketCache = Default::default();

        assert_eq!(buckets(&cache, "provider"), None);
        record(&cache, "provider", Vec::new());
        assert_eq!(buckets(&cache, "provider"), Some(Vec::new()));
    }
}
