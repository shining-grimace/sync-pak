use super::TransportSettings;
use crate::{configuration::ProviderKind, providers::endpoint};

impl TransportSettings {
    /// Resolves provider-specific routing and signing requirements without I/O.
    pub fn resolve(
        &self,
        kind: ProviderKind,
        account_id: Option<&str>,
    ) -> Result<Self, &'static str> {
        let endpoint = self.endpoint.as_deref().and_then(endpoint::normalize);
        if endpoint.as_ref().is_some_and(|endpoint| {
            !endpoint.starts_with("https://") && !endpoint.starts_with("http://")
        }) {
            return Err("S3 API endpoint must use HTTPS or HTTP.");
        }
        let region = self
            .region
            .as_deref()
            .map(str::trim)
            .filter(|region| !region.is_empty());
        let (endpoint, region) = match kind {
            ProviderKind::CloudflareR2 => {
                let endpoint = endpoint
                    .or_else(|| {
                        account_id
                            .map(str::trim)
                            .filter(|id| !id.is_empty())
                            .map(|id| format!("https://{id}.r2.cloudflarestorage.com"))
                    })
                    .ok_or("Enter the Cloudflare account ID or an S3 API endpoint.")?;
                (Some(endpoint), "auto".to_owned())
            }
            ProviderKind::AwsS3 => (
                endpoint,
                region
                    .ok_or("Choose the AWS region containing this bucket.")?
                    .to_owned(),
            ),
            ProviderKind::BackblazeB2 => {
                let endpoint = endpoint
                    .ok_or("Enter the S3 API endpoint from your Backblaze bucket's details.")?;
                if !endpoint.starts_with("https://") {
                    return Err("Backblaze S3 API endpoints require HTTPS.");
                }
                let region = match (endpoint::b2_region(&endpoint), region) {
                    (Some(inferred), Some(region)) if inferred != region => {
                        return Err("The region does not match the Backblaze S3 API endpoint.");
                    }
                    (Some(inferred), _) => inferred,
                    (None, region) => region
                        .ok_or("Enter the signing region for this custom Backblaze endpoint.")?
                        .to_owned(),
                };
                (Some(endpoint), region)
            }
        };
        Ok(Self {
            endpoint,
            region: Some(region),
        })
    }
}
