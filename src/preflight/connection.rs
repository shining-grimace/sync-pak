use crate::{
    inventory::endpoint::{EndpointPreflightError, InventoryEndpoint, collect_preflight},
    operations::request::RunRequest,
    preflight::Preflight,
    preflight::planning::Direction,
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
mod tests;
