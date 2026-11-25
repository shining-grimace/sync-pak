use crate::capabilities::{CapabilityError, CredentialBackend};

#[derive(Clone, Debug)]
pub(crate) struct CredentialProvider {
    pub backend: CredentialBackend,
    pub configured: bool,
    pub locked: bool,
}

impl CredentialProvider {
    pub(crate) fn description(&self) -> String {
        if !cfg!(target_os = "linux") {
            return self.backend.name().into();
        }
        let state = if !self.configured {
            "needs setup"
        } else if self.locked {
            "locked"
        } else {
            "unlocked"
        };
        format!("{} — {state}", self.backend.name())
    }
}

pub(crate) const BACKENDS: [CredentialBackend; 5] = [
    CredentialBackend::System,
    CredentialBackend::Kde,
    CredentialBackend::KdeLegacy6,
    CredentialBackend::KdeLegacy5,
    CredentialBackend::Gnome,
];

#[derive(Default)]
pub(crate) struct Discovery {
    #[cfg(target_os = "linux")]
    pub system_service: Option<String>,
    #[cfg(target_os = "linux")]
    pub oo7_inactive: bool,
    #[cfg(target_os = "linux")]
    pub inactive_service_addresses: Vec<&'static str>,
    pub choices: Vec<CredentialProvider>,
    pub probes: Vec<(
        CredentialBackend,
        Result<CredentialProvider, CapabilityError>,
    )>,
}

pub(crate) fn discover() -> Result<Vec<CredentialProvider>, CapabilityError> {
    discover_with_status().map(|report| report.choices)
}

pub(crate) fn status(report: &Discovery, backend: CredentialBackend) -> &'static str {
    match report
        .probes
        .iter()
        .find(|(candidate, _)| *candidate == backend)
        .map(|(_, result)| result)
    {
        Some(Ok(provider)) if !provider.configured => "needs setup",
        Some(Ok(provider)) if provider.locked => "locked",
        Some(Ok(_)) => "available",
        Some(Err(CapabilityError::AccessDenied)) => "access denied",
        Some(Err(CapabilityError::TimedOut)) => "check timed out",
        Some(Err(CapabilityError::ServiceMissing)) | None => "unavailable",
        Some(Err(_)) => "status could not be checked",
    }
}

pub(crate) fn preferred(
    providers: &[CredentialProvider],
    desktop: &str,
) -> Option<CredentialBackend> {
    providers
        .iter()
        .filter(|p| p.configured)
        .min_by_key(|p| {
            let native = if desktop.split(':').any(|d| d.eq_ignore_ascii_case("KDE")) {
                matches!(
                    p.backend,
                    CredentialBackend::Kde
                        | CredentialBackend::KdeLegacy5
                        | CredentialBackend::KdeLegacy6
                )
            } else {
                p.backend == CredentialBackend::Gnome
                    && desktop.to_ascii_lowercase().contains("gnome")
            };
            (!native, p.locked)
        })
        .map(|p| p.backend)
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::{discover_with_status, inspect};

#[cfg(not(target_os = "linux"))]
pub(crate) fn discover_with_status() -> Result<Discovery, CapabilityError> {
    let provider = CredentialProvider {
        backend: CredentialBackend::System,
        configured: true,
        locked: false,
    };
    Ok(Discovery {
        choices: vec![provider.clone()],
        probes: vec![(CredentialBackend::System, Ok(provider))],
    })
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn inspect() -> Result<Discovery, CapabilityError> {
    discover_with_status()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn card_status_uses_the_saved_address_even_when_dropdown_deduplicates_it() {
        let wallet = CredentialProvider {
            backend: CredentialBackend::Kde,
            configured: false,
            locked: false,
        };
        let report = Discovery {
            choices: vec![wallet.clone()],
            probes: vec![
                (CredentialBackend::System, Ok(wallet)),
                (CredentialBackend::Gnome, Err(CapabilityError::TimedOut)),
            ],
            ..Discovery::default()
        };
        assert_eq!(status(&report, CredentialBackend::System), "needs setup");
        assert_eq!(status(&report, CredentialBackend::Gnome), "check timed out");
        assert_eq!(
            status(&report, CredentialBackend::KdeLegacy5),
            "unavailable"
        );
    }

    #[test]
    fn plasma_prefers_its_configured_wallet_and_ignores_unconfigured_candidates() {
        let mut providers = vec![
            CredentialProvider {
                backend: CredentialBackend::Gnome,
                configured: true,
                locked: false,
            },
            CredentialProvider {
                backend: CredentialBackend::Kde,
                configured: true,
                locked: true,
            },
        ];
        assert_eq!(preferred(&providers, "KDE"), Some(CredentialBackend::Kde));
        providers[1].configured = false;
        assert_eq!(preferred(&providers, "KDE"), Some(CredentialBackend::Gnome));
        providers[0].configured = false;
        assert_eq!(preferred(&providers, "KDE"), None);
    }
}
