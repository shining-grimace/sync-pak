use std::{rc::Rc, time::Duration};

use slint::ComponentHandle;
use slint::private_unstable_api::re_exports::{ColorScheme, WindowInner};

use crate::{
    AppWindow,
    configuration::{AppearancePreference, ConfigStore},
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    preference: AppearancePreference,
) {
    apply(window, preference);
    refresh_system_appearance(window.as_weak(), preference, 20);

    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_change_appearance_preference(move |index| {
        let Some(preference) = AppearancePreference::from_index(index) else {
            return;
        };
        save(&weak, &configuration, preference);
    });
}

fn apply(window: &AppWindow, preference: AppearancePreference) {
    window.set_appearance_preference(preference.index());
    if let Some(dark_mode) = resolved_dark_mode(window, preference) {
        window.set_resolved_dark_mode(dark_mode);
    }
    window.invoke_apply_appearance_preference();
}

fn refresh_system_appearance(
    weak: slint::Weak<AppWindow>,
    preference: AppearancePreference,
    attempts_remaining: u8,
) {
    if preference != AppearancePreference::System {
        return;
    }
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        if let Some(dark_mode) = resolved_dark_mode(&window, AppearancePreference::System) {
            window.set_resolved_dark_mode(dark_mode);
            window.invoke_apply_appearance_preference();
        } else if attempts_remaining > 1 {
            refresh_system_appearance(weak, AppearancePreference::System, attempts_remaining - 1);
        }
    });
}

fn resolved_dark_mode(window: &AppWindow, preference: AppearancePreference) -> Option<bool> {
    match preference {
        AppearancePreference::Light => Some(false),
        AppearancePreference::Dark => Some(true),
        AppearancePreference::System => match WindowInner::from_pub(&window.window())
            .context()
            .color_scheme(None)
        {
            ColorScheme::Dark => Some(true),
            ColorScheme::Light => Some(false),
            _ => None,
        },
    }
}

fn save(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    preference: AppearancePreference,
) {
    let Ok(mut config) = configuration.load() else {
        show_save_error(weak);
        return;
    };
    let previous = config.appearance;
    config.appearance = preference;
    if configuration.save(&config).is_ok() {
        if let Some(window) = weak.upgrade() {
            apply(&window, preference);
            refresh_system_appearance(window.as_weak(), preference, 20);
        }
        return;
    }
    if let Some(window) = weak.upgrade() {
        apply(&window, previous);
    }
    show_save_error(weak);
}

fn show_save_error(weak: &slint::Weak<AppWindow>) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(
            "Appearance preference could not be saved. Check configuration storage and try again."
                .into(),
        );
    }
}
