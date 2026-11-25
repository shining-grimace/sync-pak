use std::{cell::Cell, rc::Rc, sync::mpsc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ConfirmationKind,
    capabilities::{CapabilityError, CredentialBackend},
    configuration::ConfigStore,
    platform::credential_discovery::{self, Discovery},
};

pub(super) fn configure(window: &AppWindow, configuration: &Rc<ConfigStore>) {
    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    let generation = Rc::new(Cell::new(0u64));
    window.on_refresh_credential_recovery_details(move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_credential_recovery_details_busy()
            || !window.get_credential_recovery_details_open()
            || !is_recovery_prompt(window.get_confirmation())
        {
            return;
        }
        let id = generation.get().wrapping_add(1);
        generation.set(id);
        window.set_credential_recovery_details_busy(true);
        window.set_credential_recovery_destination_ready(false);
        window.set_credential_recovery_detail_rows(Default::default());
        let preferred = configuration
            .load()
            .ok()
            .and_then(|config| config.credential_backend);
        let (sender, receiver) = mpsc::sync_channel(1);
        let _ = std::thread::Builder::new()
            .name("credential-details".into())
            .spawn(move || {
                let _ = sender.send(credential_discovery::inspect());
            });
        poll(
            weak.clone(),
            Rc::clone(&generation),
            id,
            preferred,
            receiver,
        );
    });
}

fn poll(
    weak: slint::Weak<AppWindow>,
    generation: Rc<Cell<u64>>,
    id: u64,
    preferred: Option<CredentialBackend>,
    receiver: mpsc::Receiver<Result<Discovery, CapabilityError>>,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if generation.get() != id || !window.get_credential_recovery_details_open() {
            return;
        }
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Disconnected) => Err(CapabilityError::Unavailable),
            Err(mpsc::TryRecvError::Empty) => {
                poll(weak, generation, id, preferred, receiver);
                return;
            }
        };
        let rows = match result {
            Ok(report) => {
                let destination = super::recovery::replacement_store(preferred, &report.choices);
                window.set_credential_recovery_destination_ready(destination.is_some());
                describe(&report, destination)
            }
            Err(_) => vec!["SyncPak could not check the credential stores in this session. Try refreshing the check.".into()],
        };
        window.set_credential_recovery_detail_rows(ModelRc::new(Rc::new(VecModel::from(
            rows.into_iter().map(SharedString::from).collect::<Vec<_>>(),
        ))));
        window.set_credential_recovery_details_busy(false);
    });
}

fn describe(report: &Discovery, destination: Option<CredentialBackend>) -> Vec<String> {
    let mut rows = Vec::new();
    #[cfg(target_os = "linux")]
    {
        let system = report
            .probes
            .iter()
            .find(|(backend, _)| *backend == CredentialBackend::System);
        let named = system
            .and_then(|(_, result)| result.as_ref().ok())
            .map(|p| p.backend);
        let owner = report.system_service.as_deref().or_else(|| {
            named
                .filter(|backend| *backend != CredentialBackend::System)
                .map(CredentialBackend::name)
        });
        let status = credential_discovery::status(report, CredentialBackend::System);
        rows.push(match owner {
            Some(owner) => format!("System Default: {owner} — {status}."),
            None => format!("System Default: {status}."),
        });
        if named.is_some_and(|backend| backend != CredentialBackend::System) {
            rows.push(
                "Settings shows the named store once when it also answers for System Default."
                    .into(),
            );
        }
        for service in &report.inactive_service_addresses {
            rows.push(format!(
                "{service} is installed, but its credential service is inactive in this session."
            ));
        }
        if report.oo7_inactive {
            rows.push("oo7 is installed but its user service is inactive. You can enable it through your system’s service settings if you want to use it.".into());
        }
    }
    match destination {
        Some(backend) => {
            if cfg!(target_os = "linux") {
                let status = credential_discovery::status(report, backend);
                rows.push(format!("Replacement destination: {} — {status}.", backend.name()));
                if status == "locked" {
                    rows.push("Unlock this store when prompted to save replacement credentials.".into());
                }
            } else {
                rows.push(format!("Replacement destination: {}. SyncPak checks write access when saving.", backend.name()));
            }
            rows.push("Go back when you’re ready to continue credential recovery.".into());
        }
        None if cfg!(target_os = "linux") => {
            rows.push("No configured replacement store found.".into());
            if report.choices.iter().any(|provider| !provider.configured) {
                rows.push("Open your desktop’s credential manager and set up a default wallet or keyring. You may need to install the manager separately.".into());
            } else {
                rows.push("Install or enable a desktop credential service and set up its default wallet or keyring, then check again.".into());
            }
        }
        None => rows.push("No replacement store could be identified. Check your device’s credential storage and try again.".into()),
    }
    rows
}

pub(super) fn is_recovery_prompt(kind: ConfirmationKind) -> bool {
    matches!(
        kind,
        ConfirmationKind::RetryProviderAccess
            | ConfirmationKind::ReplaceProviderCredentials
            | ConfirmationKind::SetupProviderCredentialStore
    )
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::platform::credential_discovery::CredentialProvider;

    #[test]
    fn separates_the_default_route_from_an_inactive_alternative_and_replacement_readiness() {
        let wallet = CredentialProvider {
            backend: CredentialBackend::Kde,
            configured: false,
            locked: false,
        };
        let report = Discovery {
            system_service: Some("KDE Wallet".into()),
            oo7_inactive: true,
            choices: vec![wallet.clone()],
            probes: vec![(CredentialBackend::System, Ok(wallet))],
            ..Discovery::default()
        };
        let rows = describe(&report, None).join("\n");
        assert!(rows.contains("System Default: KDE Wallet — needs setup"));
        assert!(rows.contains("oo7 is installed but its user service is inactive"));
        assert!(rows.contains("No configured replacement store found"));
        assert!(!rows.contains("systemctl"));
    }
}
