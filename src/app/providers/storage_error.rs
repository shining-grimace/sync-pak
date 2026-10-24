use crate::capabilities::CapabilityError;

/// Recovery text shared by saving, loading, and deleting credentials.
pub(super) fn presentation(
    error: CapabilityError,
) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match error {
        CapabilityError::Unavailable if cfg!(target_os = "linux") => (
            "Password storage could not be reached",
            "could not connect to desktop credential storage",
            "SyncPak could not connect to your desktop's password storage. Try signing out and back in, then open SyncPak from your desktop session and retry. Your entered details have been kept.",
        ),
        CapabilityError::ServiceMissing => (
            "Password storage is not available on this desktop",
            "Secret Service is missing or cannot be started",
            "SyncPak needs a desktop password service to store credentials securely. Enable GNOME Keyring or KWallet in your desktop's settings, then try again. Your entered details have been kept.",
        ),
        CapabilityError::NotConfigured => (
            "Password storage has not been set up",
            "Secret Service has no default collection",
            "Open your desktop's password manager and create or select a default keyring, then try again. On GNOME use Passwords and Keys; on KDE use KWalletManager. Your entered details have been kept.",
        ),
        CapabilityError::Locked => (
            "Password storage is still locked",
            "credential collection remained locked after unlock",
            "Your desktop did not unlock its password storage. Open your desktop's password manager, unlock the keyring, then retry. Your entered details have been kept.",
        ),
        CapabilityError::AccessDenied => (
            "Access to password storage was denied",
            "desktop credential service denied access",
            "Your desktop or app sandbox denied access to password storage. Allow SyncPak to use the desktop password service, then retry. Your entered details have been kept.",
        ),
        CapabilityError::TimedOut => (
            "Password storage did not respond",
            "credential service request exceeded five seconds",
            "Your desktop's password service did not respond within five seconds. Check that GNOME Keyring or KWallet is running, then retry. Your entered details have been kept.",
        ),
        CapabilityError::UnlockTimedOut => (
            "Password storage was not unlocked",
            "desktop unlock prompt exceeded two minutes",
            "The desktop unlock request did not finish within two minutes. If no password dialog appeared, open your desktop's password manager and unlock the keyring, then retry. Your entered details have been kept.",
        ),
        CapabilityError::Cancelled => (
            "Credential access cancelled",
            "credential access cancelled by user",
            "Credential access was cancelled. Your entered details have been kept; you can try again when ready.",
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_have_distinct_recovery_and_human_waits_have_their_own_deadline() {
        let (_, _, missing) = presentation(CapabilityError::ServiceMissing).unwrap();
        let (_, _, setup) = presentation(CapabilityError::NotConfigured).unwrap();
        let (_, _, short) = presentation(CapabilityError::TimedOut).unwrap();
        let (_, _, unlock) = presentation(CapabilityError::UnlockTimedOut).unwrap();
        assert!(missing.contains("Enable"));
        assert!(setup.contains("default keyring"));
        assert!(short.contains("five seconds"));
        assert!(unlock.contains("two minutes"));
    }
}
