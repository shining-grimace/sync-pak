use crate::{capabilities::CapabilityError, planning::OperationPlan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionState {
    Preparing,
    Copying,
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

pub trait OperationExecutor {
    fn execute(&self, plan: &OperationPlan) -> Result<(), CapabilityError>;
    fn cancel(&self, connection_id: &str) -> Result<(), CapabilityError>;
}
