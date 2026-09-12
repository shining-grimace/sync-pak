use std::rc::Rc;

use slint::ComponentHandle;

use crate::{
    AppWindow,
    configuration::{AppConfig, ConfigStore},
};

pub(super) fn configure(window: &AppWindow, configuration: &Rc<ConfigStore>) {
    let weak = window.as_weak();
    let store = Rc::clone(configuration);
    window.on_set_connection_filter(move |filter| {
        save(&weak, &store, |config| {
            config.connection_filter = filter.clamp(0, 3)
        });
    });
    let weak = window.as_weak();
    let store = Rc::clone(configuration);
    window.on_set_connections_newest_first(move |newest_first| {
        save(&weak, &store, |config| {
            config.connections_newest_first = newest_first
        });
    });
}

fn save(weak: &slint::Weak<AppWindow>, store: &ConfigStore, update: impl FnOnce(&mut AppConfig)) {
    let Some(window) = weak.upgrade() else { return };
    let result = store.load().and_then(|mut config| {
        update(&mut config);
        store.save(&config)
    });
    if result.is_ok() {
        window.invoke_show_connections();
    } else {
        window.set_status_message(
            "Connection list preferences could not be saved. Check configuration storage and try again."
                .into(),
        );
    }
}
