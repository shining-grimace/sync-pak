use crate::{
    AppWindow, TransportPreview,
    app::transport_form,
    configuration::{ConfigStore, TransportSettings},
};
use std::rc::Rc;

pub(crate) fn configure(window: &AppWindow, configuration: &Rc<ConfigStore>) {
    let configuration = Rc::clone(configuration);
    window.on_preview_connection_transport(
        move |index, bucket, inherited, endpoint, region, _generation| {
            let config = match configuration.load() {
                Ok(config) => config,
                Err(_) => return unavailable("Provider settings could not be loaded."),
            };
            let Some(provider) = usize::try_from(index)
                .ok()
                .and_then(|index| config.providers.get(index))
            else {
                return unavailable("Select a provider to see the connection settings.");
            };
            let settings =
                (!inherited).then(|| transport_form::fields(provider.kind, &endpoint, &region));
            transport_form::preview(provider, &bucket, settings.as_ref())
        },
    );
}

fn unavailable(message: &str) -> TransportPreview {
    TransportPreview {
        kind: -1,
        summary: message.into(),
    }
}

pub(crate) fn read(
    window: &AppWindow,
    configuration: &ConfigStore,
) -> Result<Option<TransportSettings>, String> {
    let config = configuration
        .load()
        .map_err(|_| "Provider settings could not be loaded.")?;
    let provider = usize::try_from(window.get_connection_form_provider())
        .ok()
        .and_then(|index| config.providers.get(index))
        .ok_or("Choose a provider.")?;
    let custom = transport_form::connection_override(window).map(|settings| {
        transport_form::fields(
            provider.kind,
            settings.endpoint.as_deref().unwrap_or_default(),
            settings.region.as_deref().unwrap_or_default(),
        )
    });
    provider
        .options
        .transport_for_bucket(Some(&window.get_connection_form_bucket()), custom.as_ref())
        .resolve(provider.kind, provider.options.account_id.as_deref())
        .map_err(str::to_owned)?;
    Ok(custom)
}

pub(crate) fn reset(window: &AppWindow) {
    window.set_connection_form_use_provider_settings(true);
    window.set_connection_form_endpoint(Default::default());
    window.set_connection_form_region(Default::default());
}

pub(crate) fn populate(window: &AppWindow, settings: Option<&TransportSettings>) {
    window.set_connection_form_use_provider_settings(settings.is_none());
    window.set_connection_form_endpoint(
        settings
            .and_then(|settings| settings.endpoint.clone())
            .unwrap_or_default()
            .into(),
    );
    window.set_connection_form_region(
        settings
            .and_then(|settings| settings.region.clone())
            .unwrap_or_default()
            .into(),
    );
}
