use std::{error::Error, fmt};

use crate::{
    inventory::Inventory,
    operations::cancellation::CancellationToken,
    operations::execution::ExecutionResult,
    operations::transfer::modes::mirror::{
        MirrorExecutionError, MirrorTransfer, execute_confirmed_mirror,
    },
    operations::transfer::progress::TransferProgressObserver,
    preflight::Preflight,
    preflight::destructive_confirmation::DestructiveConfirmation,
};

/// The current source and destination inventories checked against the reviewed plan.
pub struct CurrentInventories<'a> {
    pub source: &'a Inventory,
    pub destination: &'a Inventory,
}

/// Executes a confirmed mirror plan only while both reviewed inventories remain unchanged.
pub async fn execute_current_confirmed_mirror<
    T: MirrorTransfer,
    O: TransferProgressObserver + ?Sized,
>(
    preflight: &Preflight,
    current: CurrentInventories<'_>,
    confirmation: Option<&DestructiveConfirmation>,
    transfer: &T,
    cancellation: &CancellationToken,
    observer: &O,
    jitter_seed: u64,
) -> Result<ExecutionResult, CurrentMirrorExecutionError<T::Error>> {
    if !preflight.is_current(current.source, current.destination) {
        return Err(CurrentMirrorExecutionError::Stale);
    }
    execute_confirmed_mirror(
        preflight.plan(),
        confirmation,
        transfer,
        cancellation,
        observer,
        jitter_seed,
    )
    .await
    .map_err(CurrentMirrorExecutionError::Transfer)
}

#[derive(Debug)]
pub enum CurrentMirrorExecutionError<E> {
    Stale,
    Transfer(MirrorExecutionError<E>),
}

impl<E: fmt::Display> fmt::Display for CurrentMirrorExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale => formatter.write_str("The source or destination changed since review. Review the updated plan before starting."),
            Self::Transfer(error) => error.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for CurrentMirrorExecutionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Stale => None,
            Self::Transfer(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests;
