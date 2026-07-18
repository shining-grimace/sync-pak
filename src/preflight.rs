use std::error::Error;
use std::fmt;

use crate::comparison::{ComparedEntry, compare};
use crate::configuration::SyncMode;
use crate::inventory::{Inventory, RelativePath};
use crate::planning::{Direction, Endpoint, PlanError, TransferPlan, plan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseSensitivity {
    Sensitive,
    Insensitive,
}

/// The complete read-only result required before an operation may start.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preflight {
    comparison: Vec<ComparedEntry>,
    plan: TransferPlan,
}

impl Preflight {
    pub fn comparison(&self) -> &[ComparedEntry] {
        &self.comparison
    }

    pub fn plan(&self) -> &TransferPlan {
        &self.plan
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
    Ok(Preflight { comparison, plan })
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
#[path = "preflight_tests.rs"]
mod tests;
