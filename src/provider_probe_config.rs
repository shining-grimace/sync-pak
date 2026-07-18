use std::env;

use crate::{
    configuration::{
        CredentialReference, ProviderConfig, ProviderCredentials, ProviderId,
        ProviderKind as ConfigProviderKind, ProviderOptions,
    },
    provider_probe::ProbeError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderKind {
    AwsS3,
    BackblazeB2,
    CloudflareR2,
}

pub(crate) struct ProbeConfig {
    pub(crate) kind: ProviderKind,
    pub(crate) access_key_id: String,
    pub(crate) bucket: String,
    pub(crate) check_bucket_listing: bool,
    pub(crate) endpoint: Option<String>,
    pub(crate) prefix: String,
    pub(crate) region: String,
    pub(crate) secret_access_key: String,
    pub(crate) session_token: Option<String>,
}

impl ProbeConfig {
    pub(crate) fn from_environment() -> Result<Self, ProbeError> {
        Self::from(|name| env::var(name).ok())
    }

    #[cfg(test)]
    pub(crate) fn from_values(read: impl Fn(&str) -> Option<String>) -> Result<Self, ProbeError> {
        Self::from(read)
    }

    fn from(read: impl Fn(&str) -> Option<String>) -> Result<Self, ProbeError> {
        let kind = provider_kind(&read)?;
        let endpoint = read("SYNCPAK_PROBE_ENDPOINT").filter(|value| !value.trim().is_empty());
        if !matches!(kind, ProviderKind::AwsS3) && endpoint.is_none() {
            return Err(ProbeError::MissingSetting("SYNCPAK_PROBE_ENDPOINT"));
        }
        let prefix = required(&read, "SYNCPAK_PROBE_PREFIX")?
            .trim_matches('/')
            .to_owned();
        if prefix.is_empty() {
            return Err(ProbeError::InvalidSetting(
                "SYNCPAK_PROBE_PREFIX; it must contain a non-slash character",
            ));
        }

        Ok(Self {
            kind,
            access_key_id: required(&read, "SYNCPAK_PROBE_ACCESS_KEY_ID")?,
            bucket: required(&read, "SYNCPAK_PROBE_BUCKET")?,
            check_bucket_listing: optional_boolean(&read, "SYNCPAK_PROBE_CHECK_BUCKET_LISTING")?,
            endpoint,
            prefix,
            region: region(&read, kind)?,
            secret_access_key: required(&read, "SYNCPAK_PROBE_SECRET_ACCESS_KEY")?,
            session_token: read("SYNCPAK_PROBE_SESSION_TOKEN")
                .filter(|value| !value.trim().is_empty()),
        })
    }

    pub(crate) fn provider(&self) -> ProviderConfig {
        let id = ProviderId::new();
        ProviderConfig {
            credential_reference: CredentialReference {
                provider_id: id.clone(),
            },
            id,
            name: "Provider conformance probe".to_owned(),
            kind: match self.kind {
                ProviderKind::AwsS3 => ConfigProviderKind::AwsS3,
                ProviderKind::BackblazeB2 => ConfigProviderKind::BackblazeB2,
                ProviderKind::CloudflareR2 => ConfigProviderKind::CloudflareR2,
            },
            options: ProviderOptions {
                account_id: None,
                default_bucket: Some(self.bucket.clone()),
                endpoint: self.endpoint.clone(),
                region: Some(self.region.clone()),
            },
        }
    }

    pub(crate) fn credentials(&self) -> ProviderCredentials {
        ProviderCredentials {
            access_key_id: self.access_key_id.clone(),
            secret_access_key: self.secret_access_key.clone(),
            session_token: self.session_token.clone(),
        }
    }
}

fn provider_kind(read: &impl Fn(&str) -> Option<String>) -> Result<ProviderKind, ProbeError> {
    match required(read, "SYNCPAK_PROBE_PROVIDER")?.as_str() {
        "aws-s3" => Ok(ProviderKind::AwsS3),
        "backblaze-b2" => Ok(ProviderKind::BackblazeB2),
        "cloudflare-r2" => Ok(ProviderKind::CloudflareR2),
        _ => Err(ProbeError::InvalidSetting(
            "SYNCPAK_PROBE_PROVIDER; use aws-s3, backblaze-b2, or cloudflare-r2",
        )),
    }
}

fn required(
    read: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<String, ProbeError> {
    read(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or(ProbeError::MissingSetting(name))
}

fn optional_boolean(
    read: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<bool, ProbeError> {
    match read(name).as_deref().map(str::trim) {
        None | Some("") | Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(_) => Err(ProbeError::InvalidSetting(name)),
    }
}

fn region(
    read: &impl Fn(&str) -> Option<String>,
    kind: ProviderKind,
) -> Result<String, ProbeError> {
    match read("SYNCPAK_PROBE_REGION").filter(|value| !value.trim().is_empty()) {
        Some(region) => Ok(region),
        None => match kind {
            ProviderKind::CloudflareR2 => Ok("auto".to_owned()),
            ProviderKind::AwsS3 => Ok("us-east-1".to_owned()),
            ProviderKind::BackblazeB2 => Err(ProbeError::MissingSetting(
                "SYNCPAK_PROBE_REGION; use the region from the B2 endpoint",
            )),
        },
    }
}
