use crate::{
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    operations::execution::{ExecutionProgress, ExecutionState},
    preflight::{CaseSensitivity, preflight},
};

use super::*;

fn request(direction: Direction) -> RunRequest {
    let mut config = crate::configuration::lists::tests::config();
    config.providers[0].kind = ProviderKind::AwsS3;
    config.connections[0].mode = SyncMode::Mirror;
    config.connections[0].allow_download = direction != Direction::Upload;
    config.connections[0].allow_upload = direction != Direction::Download;
    RunRequest::widget(&config, config.connections[0].id.as_str(), true).unwrap()
}

fn collect(direction: Direction, source: &[(&str, u64)], destination: &[(&str, u64)]) -> Preflight {
    let inventory = |files: &[(&str, u64)]| {
        Inventory::new(files.iter().map(|(path, size)| {
            InventoryEntry::new(
                RelativePath::new(*path).unwrap(),
                InventoryEntryKind::File,
                *size,
                Some(1),
            )
        }))
        .unwrap()
    };
    preflight(
        SyncMode::Mirror,
        direction,
        &inventory(source),
        CaseSensitivity::Sensitive,
        &inventory(destination),
        CaseSensitivity::Sensitive,
    )
    .unwrap()
}

#[test]
fn two_way_widget_keeps_new_files_on_both_sides_without_confirmation() {
    let request = request(Direction::BothWays);
    let result = authorize(
        &request,
        collect(request.direction, &[("local", 1)], &[("remote", 1)]),
    )
    .unwrap();
    let preflight = result.preflight();
    assert!(preflight.destructive_confirmation().is_none());
    let actions = preflight.preflight().plan().actions();
    assert!(matches!(actions, [
        PlannedAction::Copy { path: local, from: Endpoint::Source, to: Endpoint::Destination },
        PlannedAction::Copy { path: remote, from: Endpoint::Destination, to: Endpoint::Source },
    ] if local.as_str() == "local" && remote.as_str() == "remote"));
}

#[test]
fn destructive_or_unresolved_mirror_blocks_all_additions() {
    for (direction, destination) in [
        (Direction::Upload, &[("extra", 1)][..]),
        (Direction::Upload, &[("shared", 2)][..]),
        (Direction::BothWays, &[("shared", 2)][..]),
    ] {
        let request = request(direction);
        let result = authorize(
            &request,
            collect(direction, &[("addition", 1), ("shared", 1)], destination),
        )
        .unwrap_err();
        assert_eq!(result.state, ExecutionState::ReviewRequired);
        assert!(result.completed.is_empty());
        assert!(result.summary().contains("No files changed"));
    }
}

#[test]
fn undocumented_endpoint_can_download_but_cannot_upload() {
    let mut request = request(Direction::BothWays);
    request.provider.options.endpoint = Some("https://custom.example".into());
    assert!(authorize(&request, collect(request.direction, &[], &[("remote", 1)])).is_ok());
    let result = authorize(
        &request,
        collect(request.direction, &[("local", 1)], &[("remote", 1)]),
    )
    .unwrap_err();
    assert_eq!(result.state, ExecutionState::ReviewRequired);
    assert!(
        result
            .summary()
            .contains("cannot guarantee create-only uploads")
    );
}

#[test]
fn destination_collision_stops_the_run_and_preserves_completed_additions() {
    let request = request(Direction::Upload);
    let plan = collect(request.direction, &[("first", 1), ("second", 1)], &[]);
    let mut progress = ExecutionProgress::new(plan.plan().actions().iter().cloned());
    progress.start_next();
    progress.complete_current();
    progress.start_next();
    let result = action_result(&request, "collision", progress.fail(), true);
    assert_eq!(result.state, ExecutionState::ReviewRequired);
    assert_eq!(result.completed.len(), 1);
    assert_eq!(result.not_started.len(), 1);
    assert!(result.incomplete.is_empty());
}
