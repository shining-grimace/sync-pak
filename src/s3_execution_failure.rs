use crate::{
    capabilities::CapabilityError,
    execution::ExecutionResult,
    inventory_endpoint::{EndpointInventoryError, EndpointPreflightError},
    provider_capabilities::ProviderError,
    provider_verification_failure::VerificationFailure,
    remote_inventory::RemoteInventoryError,
};

pub(crate) fn credential_store_message(error: CapabilityError) -> &'static str {
    match error {
        CapabilityError::Busy => {
            "Protected credential storage was busy when the run started. Wait a moment, then try again."
        }
        CapabilityError::Unsupported => {
            "Protected credential storage is not supported by this Android build."
        }
        CapabilityError::Unavailable => {
            "Protected credential storage was unavailable when the run started. Unlock the device, then try again."
        }
        CapabilityError::InvalidReference | CapabilityError::NotFound => {
            "The saved provider credential reference is no longer available. Edit and verify the provider again."
        }
        CapabilityError::UnsupportedPath | CapabilityError::Unexpected => {
            "SyncPak could not initialise protected credential storage for this run. Provider verification may still have used an earlier store instance; open Diagnostics and report this execution-stage failure."
        }
    }
}

pub(crate) fn inventory_message(error: EndpointPreflightError) -> &'static str {
    match error {
        EndpointPreflightError::SourceInventory(EndpointInventoryError::Local(_))
        | EndpointPreflightError::DestinationInventory(EndpointInventoryError::Local(_)) => {
            "SyncPak could not reread the configured local folder immediately before copying. Check the folder permission, then refresh the review."
        }
        EndpointPreflightError::SourceInventory(EndpointInventoryError::Remote(
            RemoteInventoryError::Provider(error),
        ))
        | EndpointPreflightError::DestinationInventory(EndpointInventoryError::Remote(
            RemoteInventoryError::Provider(error),
        )) => provider_message(error),
        EndpointPreflightError::SourceInventory(EndpointInventoryError::Remote(_))
        | EndpointPreflightError::DestinationInventory(EndpointInventoryError::Remote(_)) => {
            "SyncPak could not reread the cloud folder immediately before copying. Check the provider connection, then refresh the review."
        }
        EndpointPreflightError::Preflight(_) => {
            "SyncPak reread both folders but could not rebuild the transfer plan. Refresh the review and try again."
        }
    }
}

pub(crate) fn provider_message(error: ProviderError) -> &'static str {
    VerificationFailure::from(error).message()
}

pub(crate) fn failed_action(
    error: impl std::fmt::Display,
    mut result: ExecutionResult,
) -> ExecutionResult {
    result.failure_message = Some(format!("The transfer failed: {error}"));
    result
}

#[cfg(test)]
mod tests {
    use crate::{
        capabilities::CapabilityError,
        execution::{ExecutionProgress, ExecutionState},
        inventory::RelativePath,
        planning::{Endpoint, PlannedAction},
    };

    use super::{credential_store_message, failed_action};

    #[test]
    fn unexpected_credential_failure_names_the_execution_stage() {
        let message = credential_store_message(CapabilityError::Unexpected);

        assert!(message.contains("protected credential storage for this run"));
    }

    #[test]
    fn action_failure_preserves_progress_and_reports_the_cause() {
        let action = PlannedAction::Copy {
            path: RelativePath::new("photo.jpg").unwrap(),
            from: Endpoint::Source,
            to: Endpoint::Destination,
        };
        let mut progress = ExecutionProgress::new([action.clone()]);
        assert_eq!(progress.start_next(), Some(&action));

        let result = failed_action("could not write the destination", progress.fail());

        assert_eq!(result.state, ExecutionState::Failed);
        assert_eq!(result.incomplete, [action]);
        assert_eq!(
            result.failure_message.as_deref(),
            Some("The transfer failed: could not write the destination")
        );
    }
}
