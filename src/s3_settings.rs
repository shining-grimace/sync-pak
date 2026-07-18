use crate::{
    configuration::{ProviderConfig, ProviderKind},
    provider_capabilities::{ProviderError, ProviderResult},
};

pub(crate) struct S3Settings {
    pub(crate) endpoint: Option<String>,
    pub(crate) force_path_style: bool,
    pub(crate) region: String,
    pub(crate) request_checksums_when_required: bool,
}

impl S3Settings {
    pub(crate) fn from_provider(provider: &ProviderConfig) -> ProviderResult<Self> {
        let endpoint = match provider.kind {
            ProviderKind::AwsS3 => provider.options.endpoint.clone(),
            ProviderKind::CloudflareR2 => provider.options.endpoint.clone().or_else(|| {
                provider
                    .options
                    .account_id
                    .as_deref()
                    .map(|account_id| format!("https://{account_id}.r2.cloudflarestorage.com"))
            }),
            ProviderKind::BackblazeB2 => Some(
                provider
                    .options
                    .endpoint
                    .clone()
                    .ok_or(ProviderError::InvalidRequest)?,
            ),
        };
        let region = match provider.kind {
            ProviderKind::CloudflareR2 => "auto".to_owned(),
            ProviderKind::AwsS3 | ProviderKind::BackblazeB2 => provider
                .options
                .region
                .clone()
                .ok_or(ProviderError::InvalidRequest)?,
        };
        if matches!(provider.kind, ProviderKind::CloudflareR2) && endpoint.is_none() {
            return Err(ProviderError::InvalidRequest);
        }
        Ok(Self {
            endpoint,
            force_path_style: !matches!(provider.kind, ProviderKind::AwsS3),
            region,
            request_checksums_when_required: matches!(provider.kind, ProviderKind::CloudflareR2),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::S3Settings;
    use crate::configuration::{
        CredentialReference, ProviderConfig, ProviderId, ProviderKind, ProviderOptions,
    };

    #[test]
    fn r2_uses_its_account_endpoint_and_auto_region() {
        let settings = S3Settings::from_provider(&provider(
            ProviderKind::CloudflareR2,
            Some("account"),
            None,
            None,
        ))
        .unwrap();

        assert_eq!(
            settings.endpoint.as_deref(),
            Some("https://account.r2.cloudflarestorage.com")
        );
        assert_eq!(settings.region, "auto");
        assert!(settings.force_path_style);
        assert!(settings.request_checksums_when_required);
    }

    #[test]
    fn b2_requires_an_explicit_regional_endpoint() {
        assert!(
            S3Settings::from_provider(&provider(
                ProviderKind::BackblazeB2,
                None,
                None,
                Some("us-west-001")
            ))
            .is_err()
        );
    }

    fn provider(
        kind: ProviderKind,
        account_id: Option<&str>,
        endpoint: Option<&str>,
        region: Option<&str>,
    ) -> ProviderConfig {
        let id = ProviderId::new();
        ProviderConfig {
            credential_reference: CredentialReference {
                provider_id: id.clone(),
            },
            id,
            name: "Test".to_owned(),
            kind,
            options: ProviderOptions {
                account_id: account_id.map(ToOwned::to_owned),
                default_bucket: None,
                endpoint: endpoint.map(ToOwned::to_owned),
                region: region.map(ToOwned::to_owned),
            },
        }
    }
}
