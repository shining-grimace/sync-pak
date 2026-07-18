use crate::configuration::{ConfigStore, ProviderId, ProviderKind, ProviderOptions};

pub(crate) fn provider_kind(index: i32) -> Option<ProviderKind> {
    match index {
        0 => Some(ProviderKind::CloudflareR2),
        1 => Some(ProviderKind::BackblazeB2),
        2 => Some(ProviderKind::AwsS3),
        _ => None,
    }
}

pub(crate) fn provider_kind_index(kind: ProviderKind) -> i32 {
    match kind {
        ProviderKind::CloudflareR2 => 0,
        ProviderKind::BackblazeB2 => 1,
        ProviderKind::AwsS3 => 2,
    }
}

pub(crate) fn provider_options(kind: ProviderKind) -> ProviderOptions {
    let region = match kind {
        ProviderKind::CloudflareR2 => Some("auto".to_owned()),
        ProviderKind::BackblazeB2 | ProviderKind::AwsS3 => None,
    };
    ProviderOptions {
        endpoint: None,
        region,
    }
}

pub(crate) fn provider_id(configuration: &ConfigStore, id: &str) -> Result<ProviderId, String> {
    configuration
        .load()
        .map_err(|error| error.to_string())?
        .providers
        .into_iter()
        .find(|provider| provider.id.as_str() == id)
        .map(|provider| provider.id)
        .ok_or_else(|| "The provider no longer exists.".to_owned())
}
