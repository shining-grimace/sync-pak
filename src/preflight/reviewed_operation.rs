use crate::{
    operations::request::RunRequest,
    preflight::Preflight,
    preflight::confirmed::{ConfirmedPreflight, StartError},
    preflight::planning::OperationPlan,
};

/// The exact connection and read-only plan a person has reviewed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewedOperation {
    request: RunRequest,
    preflight: Preflight,
}

impl ReviewedOperation {
    pub fn new(request: RunRequest, preflight: Preflight) -> Self {
        Self { request, preflight }
    }

    /// Consumes this review so an executor receives the plan that was actually displayed.
    pub fn confirm(self, mirror_acknowledged: bool) -> Result<ConfirmedOperation, StartError> {
        Ok(ConfirmedOperation {
            request: self.request,
            preflight: ConfirmedPreflight::from_review(self.preflight, mirror_acknowledged)?,
        })
    }
}

/// A reviewed operation ready for queue submission once an executor is attached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmedOperation {
    request: RunRequest,
    preflight: ConfirmedPreflight,
}

impl ConfirmedOperation {
    /// Saved no-preview policy authorizes this exact freshly collected plan.
    pub fn from_automatic(request: RunRequest, preflight: Preflight) -> Result<Self, StartError> {
        if (request.connection.mode == crate::configuration::SyncMode::Mirror
            && request.direction == crate::preflight::planning::Direction::BothWays)
            || request.connection.show_preview
            || request.connection.mode != preflight.plan().mode()
            || request.direction != preflight.plan().direction()
        {
            return Err(StartError::AcknowledgementRequired);
        }
        Ok(Self {
            request,
            preflight: ConfirmedPreflight::from_review(preflight, true)?,
        })
    }

    pub fn request(&self) -> &RunRequest {
        &self.request
    }

    pub fn preflight(&self) -> &ConfirmedPreflight {
        &self.preflight
    }

    /// Returns the queue identity without exposing credentials or mutable review state.
    pub fn operation_plan(&self) -> OperationPlan {
        OperationPlan::new(
            self.request.connection.id.as_str(),
            self.preflight.preflight().plan().mode(),
            self.preflight.preflight().plan().direction(),
        )
    }
}

#[cfg(test)]
mod tests;
