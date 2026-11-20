use crate::{
    operations::{execution::ExecutionResult, queue::snapshot::ActivitySnapshot},
    preflight::planning::{Direction, Endpoint, PlannedAction},
};

pub(super) fn result_details(result: &ExecutionResult, snapshot: &ActivitySnapshot) -> Vec<String> {
    let mut details = Vec::new();
    let mut skipped = 0;
    for action in &result.completed {
        if matches!(action, PlannedAction::SkipChanged { .. }) {
            skipped += 1;
        } else {
            details.push(describe(action, snapshot, true));
        }
    }
    details.extend(
        result
            .incomplete
            .iter()
            .map(|action| format!("Incomplete · {}", describe(action, snapshot, false))),
    );
    details.extend(
        result
            .not_started
            .iter()
            .map(|action| format!("Not started · {}", describe(action, snapshot, false))),
    );
    if skipped > 0 {
        let files = if skipped == 1 { "file" } else { "files" };
        details.push(format!("{skipped} {files} skipped"));
    }
    details
}

fn describe(action: &PlannedAction, snapshot: &ActivitySnapshot, completed: bool) -> String {
    let (verb, endpoint, path) = match action {
        PlannedAction::Copy { path, to, .. } => {
            (if completed { "Copied" } else { "Copy" }, to, path)
        }
        PlannedAction::Overwrite { path, to, .. } => (
            if completed {
                "Overwritten"
            } else {
                "Overwrite"
            },
            to,
            path,
        ),
        PlannedAction::Delete { path, from, .. } => {
            (if completed { "Deleted" } else { "Delete" }, from, path)
        }
        PlannedAction::CreateArchive { to, .. } => {
            let place = location(snapshot, *to);
            return if completed {
                format!("Archive created {place}")
            } else {
                format!("Create archive {place}")
            };
        }
        PlannedAction::SkipChanged { path } => {
            return format!("Skip changed file: {}", path.as_str());
        }
    };
    format!(
        "{verb} {}: {}",
        location(snapshot, *endpoint),
        path.as_str()
    )
}

fn location(snapshot: &ActivitySnapshot, endpoint: Endpoint) -> String {
    let remote_endpoint = match snapshot.direction {
        Direction::Upload | Direction::BothWays => Endpoint::Destination,
        Direction::Download => Endpoint::Source,
    };
    if endpoint == remote_endpoint {
        format!(
            "on {}",
            snapshot.provider_name.as_deref().unwrap_or("provider")
        )
    } else {
        "locally".into()
    }
}
