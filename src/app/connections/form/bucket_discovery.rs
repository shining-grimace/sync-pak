use std::{rc::Rc, sync::mpsc, time::Duration};

use slint::ComponentHandle;

use crate::{
    AppWindow, BucketDiscovery,
    app::providers::{
        bucket_cache::{self, ProviderBucketCache},
        saved_verification,
    },
    configuration::{ConfigStore, ProviderConfig},
};

use super::data::set_bucket_discovery;

pub(super) fn load(
    window: &AppWindow,
    configuration: &ConfigStore,
    provider: Option<&ProviderConfig>,
    cache: &ProviderBucketCache,
) {
    let generation = window.get_connection_provider_generation().wrapping_add(1);
    window.set_connection_provider_generation(generation);
    match discovery(provider, cache) {
        Discovery::Skip => set_bucket_discovery(window, BucketDiscovery::Unchecked, Vec::new()),
        Discovery::Cached(buckets) => {
            set_bucket_discovery(window, BucketDiscovery::Listed, buckets)
        }
        Discovery::Fetch(provider_id) => {
            set_bucket_discovery(window, BucketDiscovery::Loading, Vec::new());
            let path = configuration.path().to_owned();
            let id = provider_id.clone();
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("connection-bucket-discovery".into())
                .spawn(move || {
                    let buckets = saved_verification::verify(path, id)
                        .ok()
                        .map(|verification| verification.buckets);
                    let _ = sender.send(buckets);
                });
            if worker.is_err() {
                set_bucket_discovery(window, BucketDiscovery::Failed, Vec::new());
                return;
            }
            await_buckets(
                window.as_weak(),
                generation,
                Rc::clone(cache),
                provider_id,
                receiver,
            );
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Discovery {
    Skip,
    Cached(Vec<String>),
    Fetch(String),
}

fn discovery(provider: Option<&ProviderConfig>, cache: &ProviderBucketCache) -> Discovery {
    let Some(provider) = provider.filter(|provider| provider.options.default_bucket.is_none())
    else {
        return Discovery::Skip;
    };
    let id = provider.id.as_str();
    if let Some(buckets) = bucket_cache::buckets(cache, id) {
        Discovery::Cached(buckets)
    } else if provider.verified {
        Discovery::Fetch(id.into())
    } else {
        Discovery::Skip
    }
}

fn await_buckets(
    weak: slint::Weak<AppWindow>,
    generation: i32,
    cache: ProviderBucketCache,
    provider_id: String,
    receiver: mpsc::Receiver<Option<Vec<String>>>,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if generation != window.get_connection_provider_generation() {
            return;
        }
        match receiver.try_recv() {
            Ok(Some(buckets)) => {
                bucket_cache::record(&cache, &provider_id, buckets.clone());
                set_bucket_discovery(&window, BucketDiscovery::Listed, buckets);
            }
            Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                set_bucket_discovery(&window, BucketDiscovery::Failed, Vec::new());
            }
            Err(mpsc::TryRecvError::Empty) => {
                await_buckets(weak, generation, cache, provider_id, receiver);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::ProviderKind;

    #[test]
    fn saved_verified_b2_provider_fetches_after_the_session_cache_is_lost() {
        let mut config = crate::configuration::lists::tests::config();
        let provider = &mut config.providers[0];
        provider.kind = ProviderKind::BackblazeB2;
        provider.options.default_bucket = None;
        let cache = ProviderBucketCache::default();

        assert_eq!(
            discovery(Some(provider), &cache),
            Discovery::Fetch(provider.id.as_str().into())
        );

        let names = vec!["archive".into()];
        bucket_cache::record(&cache, provider.id.as_str(), names.clone());
        assert_eq!(discovery(Some(provider), &cache), Discovery::Cached(names));
    }

    #[test]
    fn skips_discovery_for_unverified_or_fixed_bucket_providers() {
        let mut config = crate::configuration::lists::tests::config();
        let provider = &mut config.providers[0];
        let cache = ProviderBucketCache::default();

        assert_eq!(discovery(Some(provider), &cache), Discovery::Skip);
        provider.options.default_bucket = None;
        provider.verified = false;
        assert_eq!(discovery(Some(provider), &cache), Discovery::Skip);
    }
}
