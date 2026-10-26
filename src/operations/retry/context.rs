use super::RetryPolicy;
use crate::operations::cancellation::CancellationToken;

/// Per-operation retry settings and cancellation shared by transfer requests.
pub struct RetryContext<'a, S> {
    pub policy: &'a RetryPolicy,
    pub sleeper: &'a S,
    pub jitter_seed: u64,
    pub cancellation: &'a CancellationToken,
}
