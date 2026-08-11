use std::error::Error;
use std::fmt;

pub mod comparison;
pub mod confirmed;
pub mod connection;
pub mod destructive_confirmation;
pub mod plan_summary;
pub mod planning;
pub mod review;
pub mod reviewed_operation;

use crate::configuration::SyncMode;
use crate::inventory::fingerprint::{InventoryFingerprint, fingerprint};
use crate::inventory::{Inventory, RelativePath};
use crate::preflight::comparison::{ComparedEntry, compare};
use crate::preflight::planning::{Direction, Endpoint, PlanError, TransferPlan, plan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseSensitivity {
    Sensitive,
    #[cfg(any(test, target_os = "windows"))]
    Insensitive,
}

/// The complete read-only result required before an operation may start.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preflight {
    comparison: Vec<ComparedEntry>,
    plan: TransferPlan,
    source: Inventory,
    destination: Inventory,
    source_fingerprint: InventoryFingerprint,
    destination_fingerprint: InventoryFingerprint,
}

impl Preflight {
    pub fn comparison(&self) -> &[ComparedEntry] {
        &self.comparison
    }

    pub fn plan(&self) -> &TransferPlan {
        &self.plan
    }

    /// Returns the immutable source inventory that was reviewed with this plan.
    pub fn source(&self) -> &Inventory {
        &self.source
    }

    #[cfg(test)]
    pub fn is_current(&self, source: &Inventory, destination: &Inventory) -> bool {
        self.source_fingerprint == fingerprint(source)
            && self.destination_fingerprint == fingerprint(destination)
    }
}

pub fn preflight(
    mode: SyncMode,
    direction: Direction,
    source: &Inventory,
    source_case_sensitivity: CaseSensitivity,
    destination: &Inventory,
    destination_case_sensitivity: CaseSensitivity,
) -> Result<Preflight, PreflightError> {
    validate_case_collisions(
        mode,
        direction,
        source,
        destination_case_sensitivity,
        Endpoint::Source,
    )?;
    if direction == Direction::BothWays {
        validate_case_collisions(
            mode,
            direction,
            destination,
            source_case_sensitivity,
            Endpoint::Destination,
        )?;
    }
    let comparison = compare(source, destination);
    let plan = plan(mode, direction, &comparison).map_err(PreflightError::Plan)?;
    Ok(Preflight {
        comparison,
        plan,
        source: source.clone(),
        destination: destination.clone(),
        source_fingerprint: fingerprint(source),
        destination_fingerprint: fingerprint(destination),
    })
}

fn validate_case_collisions(
    mode: SyncMode,
    direction: Direction,
    inventory: &Inventory,
    target_case_sensitivity: CaseSensitivity,
    endpoint: Endpoint,
) -> Result<(), PreflightError> {
    if mode == SyncMode::Archive
        || target_case_sensitivity == CaseSensitivity::Sensitive
        || (endpoint == Endpoint::Destination && direction != Direction::BothWays)
    {
        return Ok(());
    }
    inventory
        .case_collisions()
        .into_iter()
        .next()
        .map(|paths| Err(PreflightError::CaseCollision { endpoint, paths }))
        .unwrap_or(Ok(()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreflightError {
    CaseCollision {
        endpoint: Endpoint,
        paths: Vec<RelativePath>,
    },
    Plan(PlanError),
}

impl fmt::Display for PreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CaseCollision { endpoint, paths } => write!(
                formatter,
                "{endpoint:?} paths cannot be copied to a case-insensitive destination: {}",
                paths
                    .iter()
                    .map(RelativePath::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Plan(error) => error.fmt(formatter),
        }
    }
}

impl Error for PreflightError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod golden_tests;
#[cfg(test)]
mod tests;
