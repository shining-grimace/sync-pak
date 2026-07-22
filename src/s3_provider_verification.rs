//! Read-only verification for S3-compatible provider settings and credentials.

use crate::{
    configuration::{ProviderConfig, ProviderCredentials},
    provider_capabilities::ProviderError,
    provider_verification::{ProviderVerification, verify_provider},
    s3_transport::S3Transport,
};

/// Connects with the supplied credentials and confirms visible buckets without modifying data.
pub async fn verify_s3_provider(
    provider: &ProviderConfig,
    credentials: ProviderCredentials,
) -> Result<ProviderVerification, ProviderError> {
    let transport = S3Transport::connect(provider, credentials).await?;
    verify_provider(&transport, provider.options.default_bucket.as_deref()).await
}
