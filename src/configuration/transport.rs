mod resolution;

use serde::{Deserialize, Serialize};

use super::ProviderOptions;

/// Non-secret routing and signing settings for an S3 client.
///
/// A connection's `Some(settings)` replaces the complete inherited configuration.
/// In that case, `endpoint: None` selects the standard AWS or R2 endpoint;
/// B2 requires an endpoint. AWS requires a region, B2 can derive it
/// from its standard endpoint, and R2 always uses the signing region `auto`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

impl ProviderOptions {
    /// Explicit connection settings win, followed by matching default-bucket
    /// settings, followed by account-wide provider defaults. Never apply one
    /// bucket's settings to another bucket, including after a default changes.
    pub fn transport_for_bucket(
        &self,
        bucket: Option<&str>,
        connection: Option<&TransportSettings>,
    ) -> TransportSettings {
        connection
            .or_else(|| {
                bucket
                    .filter(|bucket| Some(*bucket) == self.default_bucket.as_deref())
                    .and(self.default_bucket_transport.as_ref())
            })
            .cloned()
            .unwrap_or_else(|| TransportSettings {
                endpoint: self.endpoint.clone(),
                region: self.region.clone(),
            })
    }
}

#[cfg(test)]
mod tests;
