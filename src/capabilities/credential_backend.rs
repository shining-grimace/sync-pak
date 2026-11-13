use serde::{Deserialize, Serialize};

/// A stable endpoint identity, never a session-specific D-Bus unique name.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialBackend {
    #[default]
    System,
    Kde,
    KdeLegacy6,
    KdeLegacy5,
    Gnome,
}

impl CredentialBackend {
    #[cfg(target_os = "linux")]
    pub(crate) fn destination(self) -> &'static str {
        match self {
            Self::System => "org.freedesktop.secrets",
            Self::Kde => "org.kde.secretservicecompat",
            Self::KdeLegacy6 => "org.kde.kwalletd6",
            Self::KdeLegacy5 => "org.kde.kwalletd5",
            Self::Gnome => "org.gnome.keyring",
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::System if cfg!(target_os = "windows") => "Windows Credential Manager",
            Self::System if cfg!(target_os = "android") => "Android Keystore",
            Self::System => "System Default",
            Self::Kde => "KDE Wallet",
            Self::KdeLegacy6 => "KDE Wallet 6",
            Self::KdeLegacy5 => "KDE Wallet 5",
            Self::Gnome => "GNOME Keyring",
        }
    }
}
