use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, ConnectionRepository},
};

pub(crate) fn configure(window: &AppWindow, configuration: &Rc<ConfigStore>) {
    let weak = window.as_weak();
    let request_config = Rc::clone(configuration);
    window.on_request_connection_delete(move |id| request_delete(&weak, &request_config, id));

    let weak = window.as_weak();
    let confirm_config = Rc::clone(configuration);
    window.on_confirm_connection_delete(move || delete_connection(&weak, &confirm_config));

    let weak = window.as_weak();
    let cancel_config = Rc::clone(configuration);
    window.on_cancel_connection_delete(move || {
        crate::connection_controller::show_connections(&weak, Rc::clone(&cancel_config));
    });
}

fn request_delete(weak: &slint::Weak<AppWindow>, configuration: &ConfigStore, id: SharedString) {
    let Some(window) = weak.upgrade() else { return };
    match connection(configuration, id.as_str()) {
        Ok(connection) => {
            window.set_pending_connection_id(connection.id.as_str().into());
            window.set_pending_connection_name(connection.name.into());
            window.set_status_message(SharedString::default());
            window.set_page(7);
        }
        Err(error) => window.set_status_message(error.into()),
    }
}

fn delete_connection(weak: &slint::Weak<AppWindow>, configuration: &Rc<ConfigStore>) {
    let Some(window) = weak.upgrade() else { return };
    let result = connection(configuration, window.get_pending_connection_id().as_str()).and_then(
        |connection| {
            ConnectionRepository::new(configuration)
                .delete(&connection.id)
                .map_err(|error| error.to_string())
        },
    );
    match result {
        Ok(_) => crate::connection_controller::show_connections(weak, Rc::clone(configuration)),
        Err(error) => window.set_status_message(error.into()),
    }
}

fn connection(
    configuration: &ConfigStore,
    id: &str,
) -> Result<crate::configuration::ConnectionConfig, String> {
    configuration
        .load()
        .map_err(|error| error.to_string())?
        .connections
        .into_iter()
        .find(|connection| connection.id.as_str() == id)
        .ok_or_else(|| "The connection no longer exists.".to_owned())
}
