use std::{future::Future, time::Duration};

use crate::capabilities::{CapabilityError, CredentialInteraction, CredentialPhase};

pub(super) const SERVICE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const UNLOCK_TIMEOUT: Duration = Duration::from_secs(120);

pub(super) async fn controlled<T>(
    interaction: &CredentialInteraction,
    timeout: Duration,
    operation: impl Future<Output = Result<T, CapabilityError>>,
) -> Result<T, CapabilityError> {
    if interaction.phase() == CredentialPhase::Cancelled {
        return Err(CapabilityError::Cancelled);
    }
    futures_lite::future::race(within(timeout, operation), async {
        loop {
            if interaction.phase() == CredentialPhase::Cancelled {
                return Err(CapabilityError::Cancelled);
            }
            async_io::Timer::after(Duration::from_millis(25)).await;
        }
    })
    .await
}

pub(super) async fn check<T>(
    interaction: &CredentialInteraction,
    operation: impl Future<Output = Result<T, secret_service::Error>>,
) -> Result<T, CapabilityError> {
    interaction.transition(CredentialPhase::Checking)?;
    controlled(interaction, SERVICE_TIMEOUT, async {
        operation.await.map_err(super::credential_errors::map_error)
    })
    .await
}

/// Drops the actual async operation on expiry; this must not wrap spawn_blocking.
pub(super) async fn within<T>(
    timeout: Duration,
    operation: impl Future<Output = Result<T, CapabilityError>>,
) -> Result<T, CapabilityError> {
    futures_lite::future::race(operation, async move {
        async_io::Timer::after(timeout).await;
        Err(CapabilityError::TimedOut)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn cancelled_requests_do_not_start_another_service_operation() {
        let interaction = CredentialInteraction::default();
        assert!(interaction.cancel());
        let polled = Cell::new(false);
        let result = async_io::block_on(controlled(&interaction, SERVICE_TIMEOUT, async {
            polled.set(true);
            Ok(())
        }));
        assert_eq!(result, Err(CapabilityError::Cancelled));
        assert!(!polled.get());
    }

    #[test]
    fn human_unlock_waits_use_a_longer_budget_than_service_calls() {
        assert_eq!(SERVICE_TIMEOUT, Duration::from_secs(5));
        assert_eq!(UNLOCK_TIMEOUT, Duration::from_secs(120));
        async_io::block_on(async {
            let interaction = CredentialInteraction::default();
            let delayed = || async {
                async_io::Timer::after(Duration::from_millis(40)).await;
                Ok(42)
            };
            assert_eq!(
                controlled(&interaction, Duration::from_millis(5), delayed()).await,
                Err(CapabilityError::TimedOut)
            );
            interaction.transition(CredentialPhase::Unlocking).unwrap();
            assert_eq!(
                controlled(&interaction, Duration::from_secs(1), delayed()).await,
                Ok(42)
            );
        });
    }

    #[test]
    fn a_pending_unlock_can_be_cancelled_without_waiting_for_its_deadline() {
        async_io::block_on(async {
            let interaction = CredentialInteraction::default();
            interaction.transition(CredentialPhase::Unlocking).unwrap();
            let cancel = interaction.clone();
            let (result, ()) = futures_lite::future::zip(
                controlled(
                    &interaction,
                    UNLOCK_TIMEOUT,
                    std::future::pending::<Result<(), CapabilityError>>(),
                ),
                async move {
                    async_io::Timer::after(Duration::from_millis(10)).await;
                    assert!(cancel.cancel());
                },
            )
            .await;
            assert_eq!(result, Err(CapabilityError::Cancelled));
        });
    }

    #[test]
    fn an_unanswered_prompt_is_dropped_and_cannot_resume_after_timeout() {
        struct PendingPrompt(Rc<Cell<bool>>);
        impl Drop for PendingPrompt {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let dropped = Rc::new(Cell::new(false));
        let prompt = PendingPrompt(Rc::clone(&dropped));
        let result = async_io::block_on(within(Duration::from_millis(10), async move {
            let _prompt = prompt;
            std::future::pending::<()>().await;
            panic!("a timed-out credential operation must never resume");
            #[allow(unreachable_code)]
            Ok(())
        }));
        assert_eq!(result, Err(CapabilityError::TimedOut));
        assert!(dropped.get());
        assert_eq!(
            async_io::block_on(within(Duration::from_secs(1), async { Ok(42) })),
            Ok(42)
        );
    }
}
