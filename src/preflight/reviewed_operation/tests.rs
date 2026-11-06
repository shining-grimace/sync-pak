use crate::{
    configuration::{
        ConnectionConfig, ConnectionId, CredentialReference, ProviderConfig, ProviderId,
        ProviderKind, ProviderOptions, SyncMode,
    },
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    operations::request::RunRequest,
    preflight::planning::Direction,
    preflight::{CaseSensitivity, preflight as build_preflight},
};

use super::{ReviewedOperation, StartError};

fn request() -> RunRequest {
    let provider_id = ProviderId::new();
    RunRequest {
        connection: ConnectionConfig {
            last_run: None,
            transport: None,
            id: ConnectionId::new(),
            name: "Photos".into(),
            provider_id: provider_id.clone(),
            bucket: "backups".into(),
            remote_path: "phone".into(),
            local_path: "/photos".into(),
            mode: SyncMode::Mirror,
            allow_upload: true,
            allow_download: true,
            show_preview: true,
            keep_last_archives: None,
            verified: false,
        },
        provider: ProviderConfig {
            id: provider_id.clone(),
            credential_reference: CredentialReference {
                backend: crate::capabilities::CredentialBackend::System,
                provider_id,
            },
            name: "Cloud".into(),
            kind: ProviderKind::AwsS3,
            options: ProviderOptions {
                default_bucket_transport: None,
                account_id: None,
                default_bucket: None,
                endpoint: None,
                region: None,
            },
            verified: false,
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
fn saved_no_preview_policy_authorizes_destructive_plans_without_acknowledgement() {
    let mut request = request();
    assert_eq!(
        super::ConfirmedOperation::from_automatic(request.clone(), preflight()),
        Err(StartError::AcknowledgementRequired)
    );
    request.connection.show_preview = false;
    let automatic = super::ConfirmedOperation::from_automatic(request, preflight()).unwrap();
    assert!(automatic.preflight().destructive_confirmation().is_some());
}

#[test]
fn automatic_mirror_rejects_both_ways_even_without_destructive_actions() {
    let mut request = request();
    request.connection.show_preview = false;
    request.direction = Direction::BothWays;
    let empty = Inventory::default();
    let preflight = build_preflight(
        SyncMode::Mirror,
        Direction::BothWays,
        &empty,
        CaseSensitivity::Sensitive,
        &empty,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    assert_eq!(
        super::ConfirmedOperation::from_automatic(request, preflight),
        Err(StartError::AcknowledgementRequired)
    );
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
    assert_eq!(
        confirmed.operation_plan().connection_id,
        request.connection.id.as_str()
    );
    assert_eq!(confirmed.operation_plan().mode, SyncMode::Mirror);
}
