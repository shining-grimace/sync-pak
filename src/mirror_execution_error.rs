use std::{error::Error, fmt};

use crate::{
    destructive_confirmation::ConfirmationError, execution::ExecutionResult,
    planning::PlannedAction,
};

#[derive(Debug)]
pub enum MirrorExecutionError<E> {
    NotMirrorPlan,
    ConfirmationRequired,
    Confirmation(ConfirmationError),
    Action {
        error: MirrorActionError<E>,
        result: ExecutionResult,
    },
}

#[derive(Debug)]
pub enum MirrorActionError<E> {
    Transfer(E),
    Unsupported(PlannedAction),
}

impl<E: fmt::Display> fmt::Display for MirrorExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMirrorPlan => formatter.write_str("this execution requires a mirror plan"),
            Self::ConfirmationRequired => {
                formatter.write_str("confirm the destructive mirror plan before starting")
            }
            Self::Confirmation(error) => error.fmt(formatter),
            Self::Action { error, .. } => error.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for MirrorExecutionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Confirmation(error) => Some(error),
            Self::Action { error, .. } => Some(error),
            Self::NotMirrorPlan | Self::ConfirmationRequired => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for MirrorActionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transfer(error) => error.fmt(formatter),
            Self::Unsupported(action) => write!(formatter, "unsupported mirror action: {action:?}"),
        }
    }
}

impl<E: Error + 'static> Error for MirrorActionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transfer(error) => Some(error),
            Self::Unsupported(_) => None,
        }
    }
}
