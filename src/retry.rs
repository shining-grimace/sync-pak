use std::{future::Future, time::Duration};

use crate::provider_capabilities::ProviderError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    max_attempts: u8,
    initial_delay: Duration,
    maximum_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            initial_delay: Duration::from_millis(250),
            maximum_delay: Duration::from_secs(4),
        }
    }
}

impl RetryPolicy {
    pub fn max_attempts(&self) -> u8 {
        self.max_attempts
    }

    pub fn delay_after_failure(
        &self,
        completed_attempts: u8,
        error: ProviderError,
        provider_delay: Option<Duration>,
        jitter_seed: u64,
    ) -> Option<RetryDelay> {
        if completed_attempts >= self.max_attempts || error != ProviderError::Unavailable {
            return None;
        }
        let delay = provider_delay
            .unwrap_or_else(|| self.backoff_with_jitter(completed_attempts, jitter_seed));
        Some(RetryDelay {
            next_attempt: completed_attempts + 1,
            delay,
        })
    }

    fn backoff_with_jitter(&self, completed_attempts: u8, jitter_seed: u64) -> Duration {
        let multiplier = 1_u32 << completed_attempts.saturating_sub(1).min(4);
        let base = self
            .initial_delay
            .saturating_mul(multiplier)
            .min(self.maximum_delay);
        let jitter_percent = 80 + jitter_seed.wrapping_mul(17).wrapping_add(13) % 41;
        Duration::from_millis((base.as_millis() * jitter_percent as u128 / 100) as u64)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryDelay {
    pub next_attempt: u8,
    pub delay: Duration,
}

/// Receives retry delays for UI progress without coupling transport to a UI toolkit.
pub trait RetryObserver {
    fn on_retry(&self, retry: RetryDelay);
}

pub struct NoopRetryObserver;

impl RetryObserver for NoopRetryObserver {
    fn on_retry(&self, _: RetryDelay) {}
}

pub trait RetrySleeper {
    fn sleep(&self, delay: Duration) -> impl Future<Output = ()> + Send;
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
