use crate::{
    AppWindow,
    configuration::ConfigStore,
    platform::credential_discovery::{self, BACKENDS, CredentialProvider, Discovery},
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, rc::Rc, sync::mpsc, time::Duration};

pub(crate) fn configure(window: &AppWindow, configuration: Rc<ConfigStore>) {
    let providers = Rc::new(RefCell::new(Vec::<CredentialProvider>::new()));
    let weak = window.as_weak();
    let choices = Rc::clone(&providers);
    let store = Rc::clone(&configuration);
    window.on_select_credential_provider(move |index| {
        let Some(window) = weak.upgrade() else { return };
        if window.get_credential_discovery_busy() || window.get_provider_storage_busy() {
            return;
        }
        let Some(backend) = choices.borrow().get(index as usize).map(|p| p.backend) else {
            return;
        };
        let result = store.load().and_then(|mut config| {
            config.credential_backend = Some(backend);
            store.save(&config)
        });
        if result.is_err() {
            window.set_status_message(
                "The credential provider preference could not be saved.".into(),
            );
        }
        display(&window, &store, &choices.borrow());
    });
    let weak = window.as_weak();
    let choices = Rc::clone(&providers);
    let store = Rc::clone(&configuration);
    window.on_refresh_credential_providers(move || {
        if let Some(window) = weak.upgrade() {
            refresh(&window, Rc::clone(&store), Rc::clone(&choices));
        }
    });
    display(window, &configuration, &[]);
    refresh(window, configuration, providers);
}

fn refresh(
    window: &AppWindow,
    configuration: Rc<ConfigStore>,
    choices: Rc<RefCell<Vec<CredentialProvider>>>,
) {
    if window.get_credential_discovery_busy() || window.get_provider_storage_busy() {
        return;
    }
    window.set_credential_discovery_busy(true);
    publish_status(window, |_| "checking…");
    window.set_credential_provider_summary("Checking this device's credential providers…".into());
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name("credential-discovery".into())
        .spawn(move || {
            let _ = sender.send(credential_discovery::discover_with_status());
        });
    if worker.is_err() {
        publish_status(window, |_| "status could not be checked");
        window.set_credential_discovery_busy(false);
        window.set_credential_provider_summary(
            "Credential providers could not be checked. Retry in Settings.".into(),
        );
        return;
    }
    poll(window.as_weak(), configuration, choices, receiver);
}

fn poll(
    weak: slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    choices: Rc<RefCell<Vec<CredentialProvider>>>,
    receiver: mpsc::Receiver<Result<Discovery, crate::capabilities::CapabilityError>>,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => {
                poll(weak, configuration, choices, receiver);
                return;
            }
            Err(_) => Err(crate::capabilities::CapabilityError::Unavailable),
        };
        window.set_credential_discovery_busy(false);
        match result {
            Ok(report) => {
                publish_status(&window, |backend| {
                    credential_discovery::status(&report, backend)
                });
                *choices.borrow_mut() = report.choices;
                let selected = credential_discovery::preferred(
                    &choices.borrow(),
                    &std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
                );
                if let Ok(mut config) = configuration.load()
                    && config.credential_backend.is_none()
                    && selected.is_some()
                {
                    config.credential_backend = selected;
                    if configuration.save(&config).is_err() {
                        window.set_status_message(
                            "The automatic credential provider selection could not be saved."
                                .into(),
                        );
                    }
                }
                display(&window, &configuration, &choices.borrow());
            }
            Err(_) => {
                publish_status(&window, |_| "status could not be checked");
                choices.borrow_mut().clear();
                display(&window, &configuration, &[]);
            }
        }
    });
}

fn display(window: &AppWindow, configuration: &ConfigStore, choices: &[CredentialProvider]) {
    let selected = configuration.load().ok().and_then(|c| c.credential_backend);
    let mut labels: Vec<SharedString> = choices.iter().map(|p| p.description().into()).collect();
    let index = selected.and_then(|backend| choices.iter().position(|p| p.backend == backend));
    let display_index = index.unwrap_or_else(|| {
        let label = match selected {
            Some(backend) => format!("{} — unavailable", backend.name()),
            None => "No default store selected".into(),
        };
        labels.push(label.into());
        labels.len() - 1
    });
    window.set_credential_provider_names(ModelRc::new(Rc::new(VecModel::from(labels))));
    window.set_credential_provider_index(display_index as i32);
    let summary = match (selected, index) {
        (Some(_), Some(index)) => format!("Credentials provider: {}.", choices[index].description()),
        (Some(backend), None) => format!("Credentials provider: {} — unavailable. Retry or choose another provider in Settings. Existing credentials stay with their original provider.", backend.name()),
        (None, _) if choices.iter().any(|p| p.configured) => "Credential providers were found, but the selection could not be saved. Check configuration storage, then retry in Settings.".into(),
        (None, _) => "No usable credential provider was found. Enable your desktop's password service and set up a default keyring, then retry in Settings.".into(),
    };
    window.set_credential_provider_summary(summary.into());
}

fn publish_status(
    window: &AppWindow,
    status: impl Fn(crate::capabilities::CredentialBackend) -> &'static str,
) {
    window.set_credential_store_statuses(ModelRc::new(Rc::new(VecModel::from(
        BACKENDS
            .into_iter()
            .map(|backend| SharedString::from(status(backend)))
            .collect::<Vec<_>>(),
    ))));
}
