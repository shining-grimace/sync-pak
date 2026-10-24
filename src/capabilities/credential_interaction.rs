use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use super::CapabilityError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum CredentialPhase {
    Checking,
    Unlocking,
    Writing,
    Cancelled,
}

/// Shared progress and cancellation for one protected-storage task.
#[derive(Clone, Default)]
pub(crate) struct CredentialInteraction(Arc<AtomicU8>);

impl CredentialInteraction {
    pub(crate) fn phase(&self) -> CredentialPhase {
        match self.0.load(Ordering::Acquire) {
            0 => CredentialPhase::Checking,
            1 => CredentialPhase::Unlocking,
            2 => CredentialPhase::Writing,
            _ => CredentialPhase::Cancelled,
        }
    }

    pub(crate) fn can_cancel(&self) -> bool {
        cfg!(target_os = "linux")
            && matches!(
                self.phase(),
                CredentialPhase::Checking | CredentialPhase::Unlocking
            )
    }

    /// Cancellation and the start of a mutation race atomically. Once writing starts,
    /// cancellation must not misreport a completed credential/configuration transaction.
    pub(crate) fn cancel(&self) -> bool {
        self.transition(CredentialPhase::Cancelled).is_ok()
            && self.phase() == CredentialPhase::Cancelled
    }

    pub(crate) fn transition(&self, next: CredentialPhase) -> Result<(), CapabilityError> {
        loop {
            let current = self.0.load(Ordering::Acquire);
            if current == CredentialPhase::Cancelled as u8 {
                return Err(CapabilityError::Cancelled);
            }
            if current == CredentialPhase::Writing as u8 {
                return Ok(());
            }
            if self
                .0
                .compare_exchange(current, next as u8, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_prevents_writing_but_cannot_interrupt_a_started_mutation() {
        let cancelled = CredentialInteraction::default();
        cancelled.transition(CredentialPhase::Unlocking).unwrap();
        assert!(cancelled.cancel());
        assert_eq!(
            cancelled.transition(CredentialPhase::Writing),
            Err(CapabilityError::Cancelled)
        );
        let writing = CredentialInteraction::default();
        writing.transition(CredentialPhase::Writing).unwrap();
        assert!(!writing.cancel());
        writing.transition(CredentialPhase::Checking).unwrap();
        assert_eq!(writing.phase(), CredentialPhase::Writing);
    }
}
