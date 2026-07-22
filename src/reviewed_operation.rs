use crate::{
    confirmed_preflight::{ConfirmedPreflight, StartError},
    preflight::Preflight,
    run_request::RunRequest,
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

    pub fn request(&self) -> &RunRequest {
        &self.request
    }

    pub fn preflight(&self) -> &Preflight {
        &self.preflight
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
    pub fn request(&self) -> &RunRequest {
        &self.request
    }

    pub fn preflight(&self) -> &ConfirmedPreflight {
        &self.preflight
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        configuration::{
            ConnectionConfig, ConnectionId, CredentialReference, ProviderConfig, ProviderId,
            ProviderKind, ProviderOptions, SyncMode,
        },
        inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
        planning::Direction,
        preflight::{CaseSensitivity, preflight as build_preflight},
        run_request::RunRequest,
    };

    use super::{ReviewedOperation, StartError};

    fn request() -> RunRequest {
        let provider_id = ProviderId::new();
        RunRequest {
            connection: ConnectionConfig {
                id: ConnectionId::new(),
                name: "Photos".into(),
                provider_id: provider_id.clone(),
                bucket: "backups".into(),
                remote_path: "phone".into(),
                local_path: "/photos".into(),
                mode: SyncMode::Mirror,
                keep_last_archives: None,
            },
            provider: ProviderConfig {
                id: provider_id.clone(),
                credential_reference: CredentialReference { provider_id },
                name: "Cloud".into(),
                kind: ProviderKind::AwsS3,
                options: ProviderOptions {
                    account_id: None,
                    default_bucket: None,
                    endpoint: None,
                    region: None,
                },
            },
            direction: Direction::Upload,
        }
    }

    fn preflight() -> crate::preflight::Preflight {
        let source = Inventory::new([InventoryEntry::new(
            RelativePath::new("changed").unwrap(),
            InventoryEntryKind::File,
            2,
            Some(1),
        )])
        .unwrap();
        let destination = Inventory::new([InventoryEntry::new(
            RelativePath::new("changed").unwrap(),
            InventoryEntryKind::File,
            1,
            Some(1),
        )])
        .unwrap();
        build_preflight(
            SyncMode::Mirror,
            Direction::Upload,
            &source,
            CaseSensitivity::Sensitive,
            &destination,
            CaseSensitivity::Sensitive,
        )
        .unwrap()
    }

    #[test]
    fn confirmation_retains_the_reviewed_request_and_plan() {
        let request = request();
        let reviewed = ReviewedOperation::new(request.clone(), preflight());

        assert_eq!(
            reviewed.clone().confirm(false),
            Err(StartError::AcknowledgementRequired)
        );
        let confirmed = reviewed.confirm(true).unwrap();
        assert_eq!(confirmed.request(), &request);
        assert_eq!(
            confirmed.preflight().preflight().plan().direction(),
            Direction::Upload
        );
        assert!(confirmed.preflight().destructive_confirmation().is_some());
    }
}
