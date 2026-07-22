use crate::{
    inventory_endpoint::{EndpointPreflightError, InventoryEndpoint, collect_preflight},
    planning::Direction,
    preflight::Preflight,
    run_request::RunRequest,
};

/// Collects a read-only preflight with endpoint order chosen from the requested direction.
pub async fn collect_connection_preflight<L: InventoryEndpoint, R: InventoryEndpoint>(
    request: &RunRequest,
    local: &L,
    remote: &R,
) -> Result<Preflight, EndpointPreflightError> {
    match request.direction {
        Direction::Download => {
            collect_preflight(request.connection.mode, request.direction, remote, local).await
        }
        Direction::Upload | Direction::BothWays => {
            collect_preflight(request.connection.mode, request.direction, local, remote).await
        }
    }
}

#[cfg(test)]
#[path = "connection_preflight_tests.rs"]
mod tests;
