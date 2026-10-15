use crate::{
    configuration::{ProviderConfig, ProviderKind, TransportSettings},
    providers::capabilities::{ProviderError, ProviderResult},
};

pub(crate) struct S3Settings {
    pub(crate) endpoint: Option<String>,
    pub(crate) force_path_style: bool,
    pub(crate) region: String,
    pub(crate) request_checksums_when_required: bool,
}

impl S3Settings {
    pub(crate) fn from_provider(provider: &ProviderConfig) -> ProviderResult<Self> {
        Self::for_bucket(provider, provider.options.default_bucket.as_deref(), None)
    }

    pub(crate) fn for_bucket(
        provider: &ProviderConfig,
        bucket: Option<&str>,
        connection: Option<&TransportSettings>,
    ) -> ProviderResult<Self> {
        let options = provider
            .options
            .transport_for_bucket(bucket, connection)
            .resolve(provider.kind, provider.options.account_id.as_deref())
            .map_err(|_| ProviderError::InvalidRequest)?;
        Ok(Self {
            endpoint: options.endpoint,
            force_path_style: !matches!(provider.kind, ProviderKind::AwsS3),
            region: options.region.ok_or(ProviderError::InvalidRequest)?,
            request_checksums_when_required: matches!(provider.kind, ProviderKind::CloudflareR2),
        })
    }
}

#[cfg(test)]
mod tests;
