use crate::{
    AppWindow, TransportPreview,
    configuration::{ProviderConfig, ProviderKind, TransportSettings},
    providers::endpoint,
};

/// Form fields may contain an old manually entered B2 region while the UI shows
/// the region inferred from the current endpoint. Save exactly what is shown.
pub(crate) fn fields(kind: ProviderKind, endpoint: &str, region: &str) -> TransportSettings {
    let endpoint = endpoint::normalize(endpoint);
    let region = match kind {
        ProviderKind::CloudflareR2 => None,
        ProviderKind::BackblazeB2 => endpoint
            .as_deref()
            .and_then(endpoint::b2_region)
            .or_else(|| optional(region)),
        ProviderKind::AwsS3 => optional(region),
    };
    TransportSettings { endpoint, region }
}

pub(crate) fn optional(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

pub(crate) fn connection_override(window: &AppWindow) -> Option<TransportSettings> {
    (!window.get_connection_form_use_provider_settings()).then(|| TransportSettings {
        endpoint: endpoint::normalize(&window.get_connection_form_endpoint()),
        region: optional(&window.get_connection_form_region()),
    })
}

pub(crate) fn preview(
    provider: &ProviderConfig,
    bucket: &str,
    connection: Option<&TransportSettings>,
) -> TransportPreview {
    let connection = connection.map(|settings| {
        fields(
            provider.kind,
            settings.endpoint.as_deref().unwrap_or_default(),
            settings.region.as_deref().unwrap_or_default(),
        )
    });
    let options = provider
        .options
        .transport_for_bucket(Some(bucket), connection.as_ref());
    let summary = match options.resolve(provider.kind, provider.options.account_id.as_deref()) {
        Ok(settings) if provider.kind == ProviderKind::CloudflareR2 => {
            format!(
                "Endpoint: {}",
                settings.endpoint.as_deref().unwrap_or_default()
            )
        }
        Ok(settings) => format!(
            "Endpoint: {}\nRegion: {}",
            settings
                .endpoint
                .as_deref()
                .unwrap_or("AWS standard endpoint"),
            settings.region.as_deref().unwrap_or_default()
        ),
        Err(error) => format!("Connection settings: {error}"),
    };
    TransportPreview {
        kind: super::providers::form::state::provider_kind_index(provider.kind),
        summary: summary.into(),
    }
}

#[cfg(test)]
mod tests;
