use std::{error::Error, fmt};

use crate::planning::TransferPlan;

/// A user confirmation bound to one immutable plan containing destructive actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DestructiveConfirmation {
    confirmed_plan: TransferPlan,
}

impl DestructiveConfirmation {
    pub fn confirm(plan: &TransferPlan) -> Result<Self, ConfirmationError> {
        if !plan.requires_confirmation() {
            return Err(ConfirmationError::NotRequired);
        }
        Ok(Self {
            confirmed_plan: plan.clone(),
        })
    }

    /// Verifies that the reviewed plan is exactly the plan the user confirmed.
    pub fn verify(&self, plan: &TransferPlan) -> Result<(), ConfirmationError> {
        if &self.confirmed_plan == plan {
            Ok(())
        } else {
            Err(ConfirmationError::PlanChanged)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationError {
    NotRequired,
    PlanChanged,
}

impl fmt::Display for ConfirmationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotRequired => "This plan has no destructive actions to confirm.",
            Self::PlanChanged => {
                "The plan changed since confirmation. Review and confirm the updated plan."
            }
        })
    }
}

impl Error for ConfirmationError {}

#[cfg(test)]
mod tests {
    use crate::{
        comparison::compare,
        configuration::SyncMode,
        inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
        planning::{Direction, plan},
    };

    use super::{ConfirmationError, DestructiveConfirmation};

    fn inventory(paths: &[&str]) -> Inventory {
        Inventory::new(paths.iter().map(|path| {
            InventoryEntry::new(
                RelativePath::new(*path).unwrap(),
                InventoryEntryKind::File,
                1,
                Some(1),
            )
        }))
        .unwrap()
    }

    #[test]
    fn confirmation_is_valid_only_for_the_exact_destructive_plan() {
        let confirmed = plan(
            SyncMode::Mirror,
            Direction::Upload,
            &compare(&inventory(&[]), &inventory(&["delete"])),
        )
        .unwrap();
        let changed = plan(
            SyncMode::Mirror,
            Direction::Upload,
            &compare(&inventory(&[]), &inventory(&["other"])),
        )
        .unwrap();
        let confirmation = DestructiveConfirmation::confirm(&confirmed).unwrap();

        assert_eq!(confirmation.verify(&confirmed), Ok(()));
        assert_eq!(
            confirmation.verify(&changed),
            Err(ConfirmationError::PlanChanged)
        );
    }

    #[test]
    fn non_destructive_plan_cannot_receive_a_destructive_confirmation() {
        let plan = plan(
            SyncMode::AddOnly,
            Direction::Upload,
            &compare(&inventory(&["copy"]), &inventory(&[])),
        )
        .unwrap();

        assert_eq!(
            DestructiveConfirmation::confirm(&plan),
            Err(ConfirmationError::NotRequired)
        );
    }
}
