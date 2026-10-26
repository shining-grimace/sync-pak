use crate::{
    AppWindow,
    app::transport_form,
    configuration::{ProviderKind, ProviderOptions},
};

pub(crate) fn options(window: &AppWindow, kind: ProviderKind) -> Result<ProviderOptions, String> {
    let defaults = transport_form::fields(
        kind,
        &window.get_provider_form_endpoint(),
        &window.get_provider_form_region(),
    );
    let bucket = transport_form::optional(&window.get_provider_form_bucket());
    let bucket_transport =
        (bucket.is_some() && window.get_provider_form_bucket_custom()).then(|| {
            transport_form::fields(
                kind,
                &window.get_provider_form_bucket_endpoint(),
                &window.get_provider_form_bucket_region(),
            )
        });
    let options = ProviderOptions {
        account_id: transport_form::optional(&window.get_provider_form_account_id()),
        default_bucket: bucket,
        default_bucket_transport: bucket_transport,
        endpoint: defaults.endpoint,
        region: defaults.region,
    };
    // Basic fields and provider defaults are validated in visual order by
    // form_validation::provider before these fixed-bucket overrides.
    if let Some(settings) = &options.default_bucket_transport {
        settings
            .resolve(kind, options.account_id.as_deref())
            .map_err(str::to_owned)?;
    }
    Ok(options)
}

pub(crate) fn reset(window: &AppWindow) {
    window.set_provider_form_bucket_custom(false);
    window.set_provider_form_bucket_endpoint(Default::default());
    window.set_provider_form_bucket_region(Default::default());
}

pub(crate) fn populate(window: &AppWindow, options: &ProviderOptions) {
    window.set_provider_form_bucket_custom(options.default_bucket_transport.is_some());
    let settings = options.default_bucket_transport.clone().unwrap_or_default();
    window.set_provider_form_bucket_endpoint(settings.endpoint.unwrap_or_default().into());
    window.set_provider_form_bucket_region(settings.region.unwrap_or_default().into());
}
