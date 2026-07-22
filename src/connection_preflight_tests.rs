use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

use crate::{
    configuration::{
        AppConfig, ConnectionConfig, ConnectionId, CredentialReference, ProviderConfig, ProviderId,
        ProviderKind, ProviderOptions, SyncMode,
    },
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    inventory_endpoint::{EndpointInventoryError, InventoryEndpoint},
    planning::Direction,
    preflight::CaseSensitivity,
    run_request::RunRequest,
};

use super::collect_connection_preflight;

struct Endpoint(Inventory);

impl InventoryEndpoint for Endpoint {
    fn case_sensitivity(&self) -> CaseSensitivity {
        CaseSensitivity::Sensitive
    }

    async fn collect(&self) -> Result<Inventory, EndpointInventoryError> {
        Ok(self.0.clone())
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the test endpoints are immediately ready"),
    }
}

fn file(path: &str) -> InventoryEntry {
    InventoryEntry::new(
        RelativePath::new(path).unwrap(),
        InventoryEntryKind::File,
        1,
        Some(1),
    )
}

fn request(direction: Direction) -> RunRequest {
    let provider_id = ProviderId::new();
    let provider = ProviderConfig {
        id: provider_id.clone(),
        name: "Cloud".into(),
        kind: ProviderKind::AwsS3,
        options: ProviderOptions {
            account_id: None,
            default_bucket: None,
            endpoint: None,
            region: Some("ap-southeast-2".into()),
        },
        credential_reference: CredentialReference { provider_id },
    };
    let connection = ConnectionConfig {
        id: ConnectionId::new(),
        name: "Photos".into(),
        provider_id: provider.id.clone(),
        bucket: "backups".into(),
        remote_path: String::new(),
        local_path: "/photos".into(),
        mode: SyncMode::AddOnly,
        keep_last_archives: None,
    };
    RunRequest::from_config(
        &AppConfig {
            providers: vec![provider],
            connections: vec![connection.clone()],
            ..Default::default()
        },
        connection.id.as_str(),
        direction,
    )
    .unwrap()
}

#[test]
fn download_inventories_remote_as_the_source() {
    let local = Endpoint(Inventory::new([file("local-only")]).unwrap());
    let remote = Endpoint(Inventory::new([file("remote-only")]).unwrap());

    let preflight = block_on(collect_connection_preflight(
        &request(Direction::Download),
        &local,
        &remote,
    ))
    .unwrap();

    let remote_entry = preflight
        .comparison()
        .iter()
        .find(|entry| entry.path.as_str() == "remote-only")
        .unwrap();
    assert!(remote_entry.source.is_some());
    assert!(remote_entry.destination.is_none());
}
