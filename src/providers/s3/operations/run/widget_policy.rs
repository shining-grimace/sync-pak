use crate::{
    configuration::{ProviderKind, SyncMode},
    operations::{execution::ExecutionResult, request::RunRequest},
    preflight::{
        Preflight,
        mirror_selection::{MirrorChoice, MirrorSelection},
        planning::{Direction, Endpoint, PlannedAction},
        reviewed_operation::{ConfirmedOperation, ReviewedOperation},
    },
};

pub(super) fn authorize(
    request: &RunRequest,
    preflight: Preflight,
) -> Result<ConfirmedOperation, ExecutionResult> {
    let review = || ExecutionResult::review_required();
    if request.connection.mode == SyncMode::Archive {
        return Err(review());
    }

    let selected = if request.connection.mode == SyncMode::Mirror
        && request.direction == Direction::BothWays
    {
        if preflight.comparison().iter().any(|entry| {
            entry.status == crate::preflight::comparison::EntryStatus::Changed
                && entry.source.is_some()
                && entry.destination.is_some()
        }) {
            return Err(review());
        }
        let mut selection = MirrorSelection::new(preflight);
        let new_paths = selection
            .original()
            .comparison()
            .iter()
            .filter(|entry| entry.source.is_none() || entry.destination.is_none())
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        for path in new_paths {
            selection.choose(&path, Some(MirrorChoice::Keep));
        }
        // Any outstanding choice, including a changed file on both sides, blocks everything.
        selection.selected().map_err(|_| review())?
    } else {
        preflight
    };

    if selected.plan().requires_confirmation() {
        return Err(review());
    }
    let uploads = selected.plan().actions().iter().any(|action| {
        matches!(
            action,
            PlannedAction::Copy { from: Endpoint::Source, .. }
                if request.direction != Direction::Download
        )
    });
    if uploads && !supports_conditional_upload(request) {
        let mut result = review();
        result.failure_message = Some(
            "Review required. No files changed. This provider endpoint cannot guarantee create-only uploads from a widget.".into()
        );
        return Err(result);
    }

    ReviewedOperation::new(request.clone(), selected)
        .confirm(false)
        .map_err(|_| review())
}

fn supports_conditional_upload(request: &RunRequest) -> bool {
    let provider = &request.provider;
    let Ok(settings) = provider
        .options
        .transport_for_bucket(
            Some(&request.connection.bucket),
            request.connection.transport.as_ref(),
        )
        .resolve(provider.kind, provider.options.account_id.as_deref())
    else {
        return false;
    };
    match (provider.kind, settings.endpoint.as_deref()) {
        (ProviderKind::AwsS3, None) => true,
        (ProviderKind::CloudflareR2, Some(endpoint)) => endpoint
            .strip_prefix("https://")
            .and_then(|host| {
                host.trim_end_matches('/')
                    .strip_suffix(".r2.cloudflarestorage.com")
            })
            .is_some_and(|prefix| {
                !prefix.is_empty()
                    && prefix
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
            }),
        _ => false,
    }
}

pub(super) fn action_result(
    request: &RunRequest,
    error: impl std::fmt::Display,
    mut result: ExecutionResult,
    collision: bool,
) -> ExecutionResult {
    if request.widget_create_only && collision {
        result.state = crate::operations::execution::ExecutionState::ReviewRequired;
        result.failure_message = Some(format!(
            "Review required. The destination changed during the run. {} additions completed. Open SyncPak to review.",
            result.completed.len()
        ));
        result.not_started.splice(0..0, result.incomplete.drain(..));
        result
    } else {
        crate::providers::s3::operations::failure::failed_action(error, result)
    }
}

#[cfg(test)]
mod tests;
