use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ProviderRow,
    configuration::{
        ConfigStore, ProviderCredentials, ProviderDraft, ProviderKind, ProviderRepository,
    },
    platform::PlatformCredentialStore,
    provider_form::{provider_id, provider_kind, provider_kind_index, provider_options},
};

pub(crate) fn initialize(window: &AppWindow) {
    let configuration = match ConfigStore::for_current_platform() {
        Ok(configuration) => Rc::new(configuration),
        Err(error) => {
            window.set_status_message(
                format!("SyncPak could not access configuration: {error}").into(),
            );
            return;
        }
    };
    configure_navigation(window, &configuration);
    configure_save_provider(window, &configuration);
    crate::provider_delete_controller::configure(window, &configuration);
    crate::connection_controller::configure(window, &configuration);
    crate::connection_delete_controller::configure(window, &configuration);
    crate::folder_picker_controller::configure(window);
}

fn configure_navigation(window: &AppWindow, configuration: &Rc<ConfigStore>) {
    let weak = window.as_weak();
    let providers_config = Rc::clone(configuration);
    window.on_show_providers(move || show_providers(&weak, Rc::clone(&providers_config)));

    let weak = window.as_weak();
    window.on_show_add_provider(move || show_add_provider(&weak));

    let weak = window.as_weak();
    let edit_config = Rc::clone(configuration);
    window.on_request_provider_edit(move |id| request_provider_edit(&weak, &edit_config, id));

    let weak = window.as_weak();
    window.on_show_welcome(move || set_page(&weak, 0));

    let weak = window.as_weak();
    window.on_show_privacy(move || set_page(&weak, 3));
}

fn configure_save_provider(window: &AppWindow, configuration: &Rc<ConfigStore>) {
    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_save_provider(move |name, kind, access_key_id, secret_access_key| {
        save_provider(
            &weak,
            Rc::clone(&configuration),
            name,
            kind,
            access_key_id,
            secret_access_key,
        );
    });
}

fn save_provider(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    name: SharedString,
    kind: i32,
    access_key_id: SharedString,
    secret_access_key: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    let Some(kind) = provider_kind(kind) else {
        window.set_status_message("Choose a provider type.".into());
        return;
    };
    let credentials = ProviderCredentials {
        access_key_id: access_key_id.to_string(),
        secret_access_key: secret_access_key.to_string(),
        session_token: None,
    };
    let draft = ProviderDraft {
        name: name.to_string(),
        kind,
        options: provider_options(kind),
    };
    let edit_id = window.get_provider_form_id();
    let result = (|| {
        let store = PlatformCredentialStore::new().map_err(|error| error.to_string())?;
        let repository = ProviderRepository::new(&configuration, &store);
        if edit_id.is_empty() {
            repository
                .create(draft, &credentials)
                .map_err(|error| error.to_string())
        } else {
            let id = provider_id(&configuration, edit_id.as_str())?;
            repository
                .update(&id, draft, &credentials)
                .map_err(|error| error.to_string())
        }
    })();
    match result {
        Ok(_) => show_providers(weak, configuration),
        Err(error) => window.set_status_message(error.into()),
    }
}

pub(crate) fn show_providers(weak: &slint::Weak<AppWindow>, configuration: Rc<ConfigStore>) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(1);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        refresh_providers(&weak, &configuration)
    });
}

fn refresh_providers(weak: &slint::Weak<AppWindow>, configuration: &ConfigStore) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            let rows = config.providers.into_iter().map(|provider| ProviderRow {
                id: provider.id.as_str().into(),
                name: provider.name.into(),
                kind: provider_kind_name(provider.kind).into(),
            });
            window.set_providers(ModelRc::new(Rc::new(VecModel::from_iter(rows))));
            window.set_status_message(SharedString::default());
        }
        Err(error) => {
            window.set_status_message(format!("SyncPak could not load providers: {error}").into())
        }
    }
}

fn show_add_provider(weak: &slint::Weak<AppWindow>) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(SharedString::default());
        window.set_provider_form_id(SharedString::default());
        window.set_provider_form_name(SharedString::default());
        window.set_provider_form_kind(0);
        window.set_page(2);
    }
}

fn request_provider_edit(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration
        .load()
        .map_err(|error| error.to_string())
        .and_then(|config| {
            config
                .providers
                .into_iter()
                .find(|provider| id == provider.id.as_str())
                .ok_or_else(|| "The provider no longer exists.".to_owned())
        }) {
        Ok(provider) => {
            window.set_provider_form_id(provider.id.as_str().into());
            window.set_provider_form_name(provider.name.into());
            window.set_provider_form_kind(provider_kind_index(provider.kind));
            window.set_status_message(SharedString::default());
            window.set_page(2);
        }
        Err(error) => window.set_status_message(error.into()),
    }
}

fn set_page(weak: &slint::Weak<AppWindow>, page: i32) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(SharedString::default());
        window.set_page(page);
    }
}

fn provider_kind_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::CloudflareR2 => "Cloudflare R2",
        ProviderKind::BackblazeB2 => "Backblaze B2",
        ProviderKind::AwsS3 => "AWS S3",
    }
}
