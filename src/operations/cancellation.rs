use std::{
    error::Error,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// A shared signal that stops an operation before its next safe boundary.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    requested: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Requests cancellation for every clone of this token.
    pub fn cancel(&self) {
        self.requested.store(true, Ordering::Release);
    }

    /// Returns whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }

    /// Returns an error when the caller should stop before starting more work.
    pub fn check(&self) -> Result<(), Cancelled> {
        (!self.is_cancelled()).then_some(()).ok_or(Cancelled)
    }
}

/// Indicates that an operation stopped in response to a cancellation request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operation was cancelled")
    }
}

impl Error for Cancelled {}

#[cfg(test)]
mod tests {
    use super::{CancellationToken, Cancelled};

    #[test]
    fn clones_observe_the_same_cancellation_request() {
        let token = CancellationToken::default();
        let clone = token.clone();

        token.cancel();

        assert!(clone.is_cancelled());
    }

    #[test]
    fn check_reports_a_cancelled_operation() {
        let token = CancellationToken::default();
        assert_eq!(token.check(), Ok(()));

        token.cancel();

        assert_eq!(token.check(), Err(Cancelled));
    }
}
