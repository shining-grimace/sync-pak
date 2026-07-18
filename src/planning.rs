use std::error::Error;
use std::fmt;

use crate::comparison::{ComparedEntry, EntryStatus};
use crate::configuration::SyncMode;
use crate::inventory::RelativePath;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Upload,
    Download,
    BothWays,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationPlan {
    pub connection_id: String,
    pub mode: SyncMode,
    pub direction: Direction,
}

impl OperationPlan {
    pub fn new(connection_id: impl Into<String>, mode: SyncMode, direction: Direction) -> Self {
        Self {
            connection_id: connection_id.into(),
            mode,
            direction,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endpoint {
    Source,
    Destination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannedAction {
    Copy {
        path: RelativePath,
        from: Endpoint,
        to: Endpoint,
    },
    Overwrite {
        path: RelativePath,
        from: Endpoint,
        to: Endpoint,
    },
    Delete {
        path: RelativePath,
        from: Endpoint,
    },
    SkipChanged {
        path: RelativePath,
    },
    CreateArchive {
        from: Endpoint,
        to: Endpoint,
    },
}

impl PlannedAction {
    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::Overwrite { .. } | Self::Delete { .. })
    }
}

/// A read-only, immutable preview of the work an operation will perform.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferPlan {
    mode: SyncMode,
    direction: Direction,
    actions: Vec<PlannedAction>,
}

impl TransferPlan {
    pub fn mode(&self) -> SyncMode {
        self.mode
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn actions(&self) -> &[PlannedAction] {
        &self.actions
    }

    pub fn requires_confirmation(&self) -> bool {
        self.actions.iter().any(PlannedAction::is_destructive)
    }
}

pub fn plan(
    mode: SyncMode,
    direction: Direction,
    comparison: &[ComparedEntry],
) -> Result<TransferPlan, PlanError> {
    if direction == Direction::BothWays && mode != SyncMode::AddOnly {
        return Err(PlanError::BothWaysRequiresAddOnly);
    }
    let actions = match mode {
        SyncMode::AddOnly => plan_add_only(direction, comparison),
        SyncMode::Mirror => plan_mirror(comparison),
        SyncMode::Archive => vec![PlannedAction::CreateArchive {
            from: Endpoint::Source,
            to: Endpoint::Destination,
        }],
    };
    Ok(TransferPlan {
        mode,
        direction,
        actions,
    })
}

fn plan_add_only(direction: Direction, comparison: &[ComparedEntry]) -> Vec<PlannedAction> {
    comparison
        .iter()
        .filter_map(
            |entry| match (entry.source.is_some(), entry.destination.is_some()) {
                (true, false) => Some(copy(&entry.path, Endpoint::Source, Endpoint::Destination)),
                (false, true) if direction == Direction::BothWays => {
                    Some(copy(&entry.path, Endpoint::Destination, Endpoint::Source))
                }
                (true, true) if entry.status == EntryStatus::Changed => {
                    Some(PlannedAction::SkipChanged {
                        path: entry.path.clone(),
                    })
                }
                _ => None,
            },
        )
        .collect()
}

fn plan_mirror(comparison: &[ComparedEntry]) -> Vec<PlannedAction> {
    comparison
        .iter()
        .filter_map(
            |entry| match (entry.source.is_some(), entry.destination.is_some()) {
                (true, false) => Some(copy(&entry.path, Endpoint::Source, Endpoint::Destination)),
                (false, true) => Some(PlannedAction::Delete {
                    path: entry.path.clone(),
                    from: Endpoint::Destination,
                }),
                (true, true) if entry.status == EntryStatus::Changed => {
                    Some(PlannedAction::Overwrite {
                        path: entry.path.clone(),
                        from: Endpoint::Source,
                        to: Endpoint::Destination,
                    })
                }
                _ => None,
            },
        )
        .collect()
}

fn copy(path: &RelativePath, from: Endpoint, to: Endpoint) -> PlannedAction {
    PlannedAction::Copy {
        path: path.clone(),
        from,
        to,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanError {
    BothWaysRequiresAddOnly,
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "both-way operations are only available in add-only mode"
        )
    }
}

impl Error for PlanError {}

#[cfg(test)]
#[path = "planning_tests.rs"]
mod tests;
