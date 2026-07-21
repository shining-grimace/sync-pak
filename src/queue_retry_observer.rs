use uuid::Uuid;

use crate::{
    operation_progress::RetryStatus,
    retry::{RetryDelay, RetryObserver, RetryPolicy},
};

/// Translates a transport retry into a queue-facing retry status update.
pub struct QueueRetryObserver<F> {
    operation_id: Uuid,
    max_attempts: u8,
    publish: F,
}

impl<F> QueueRetryObserver<F> {
    pub fn new(operation_id: Uuid, policy: &RetryPolicy, publish: F) -> Self {
        Self {
            operation_id,
            max_attempts: policy.max_attempts(),
            publish,
        }
    }
}

impl<F: Fn(Uuid, RetryStatus)> RetryObserver for QueueRetryObserver<F> {
    fn on_retry(&self, retry: RetryDelay) {
        (self.publish)(
            self.operation_id,
            RetryStatus {
                next_attempt: retry.next_attempt,
                max_attempts: self.max_attempts,
                delay_millis: retry.delay.as_millis().try_into().unwrap_or(u64::MAX),
            },
        );
    }
}
