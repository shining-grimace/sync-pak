use std::{error::Error, fmt};

use crate::{
    add_only_execution::{AddOnlyExecutionError, AddOnlyTransfer, execute_add_only_actions},
    cancellation::CancellationToken,
    execution::ExecutionResult,
    inventory::Inventory,
    preflight::Preflight,
    transfer_progress::TransferProgressObserver,
};

/// Runs an add-only preflight only when the inventories still match its confirmation.
pub async fn execute_current_add_only<T: AddOnlyTransfer, O: TransferProgressObserver>(
    preflight: &Preflight,
    source: &Inventory,
    destination: &Inventory,
    transfer: &T,
    cancellation: &CancellationToken,
    observer: &O,
    jitter_seed: u64,
) -> Result<ExecutionResult, PreflightExecutionError<T::Error>> {
    if !preflight.is_current(source, destination) {
        return Err(PreflightExecutionError::Stale);
    }
    execute_add_only_actions(
        preflight.plan().direction(),
        preflight.plan().actions(),
        transfer,
        cancellation,
        observer,
        jitter_seed,
    )
    .await
    .map_err(PreflightExecutionError::Transfer)
}

#[derive(Debug)]
pub enum PreflightExecutionError<E> {
    Stale,
    Transfer(AddOnlyExecutionError<E>),
}

impl<E: fmt::Display> fmt::Display for PreflightExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale => formatter.write_str(
                "The source or destination changed since review. Review the updated plan before starting.",
            ),
            Self::Transfer(error) => error.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for PreflightExecutionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Stale => None,
            Self::Transfer(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "preflight_execution_tests.rs"]
mod tests;
