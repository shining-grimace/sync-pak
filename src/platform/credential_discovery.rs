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
pub(crate) use linux::discover;

#[cfg(not(target_os = "linux"))]
pub(crate) fn discover() -> Result<Vec<CredentialProvider>, CapabilityError> {
    Ok(vec![CredentialProvider {
        backend: CredentialBackend::System,
        configured: true,
        locked: false,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
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
