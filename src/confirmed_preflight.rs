use std::{error::Error, fmt};

use crate::{destructive_confirmation::DestructiveConfirmation, preflight::Preflight};

/// An immutable preflight that is eligible to be handed to an operation executor.
///
/// Destructive plans retain confirmation for exactly the plan that was reviewed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmedPreflight {
    preflight: Preflight,
    destructive_confirmation: Option<DestructiveConfirmation>,
}

impl ConfirmedPreflight {
    pub fn from_review(
        preflight: Preflight,
        mirror_acknowledged: bool,
    ) -> Result<Self, StartError> {
        let destructive_confirmation = if preflight.plan().requires_confirmation() {
            if !mirror_acknowledged {
                return Err(StartError::AcknowledgementRequired);
            }
            Some(
                DestructiveConfirmation::confirm(preflight.plan())
                    .map_err(StartError::Confirmation)?,
            )
        } else {
            None
        };
        Ok(Self {
            preflight,
            destructive_confirmation,
        })
    }

    pub fn preflight(&self) -> &Preflight {
        &self.preflight
    }

    pub fn destructive_confirmation(&self) -> Option<&DestructiveConfirmation> {
        self.destructive_confirmation.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartError {
    AcknowledgementRequired,
    Confirmation(crate::destructive_confirmation::ConfirmationError),
}

impl fmt::Display for StartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AcknowledgementRequired => formatter.write_str(
                "Review and acknowledge the listed overwrites and deletions before starting.",
            ),
            Self::Confirmation(error) => error.fmt(formatter),
        }
    }
}

impl Error for StartError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AcknowledgementRequired => None,
            Self::Confirmation(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        configuration::SyncMode,
        inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
        planning::Direction,
        preflight::{CaseSensitivity, preflight},
    };

    use super::{ConfirmedPreflight, StartError};

    fn inventory(path: &str, size: u64) -> Inventory {
        Inventory::new([InventoryEntry::new(
            RelativePath::new(path).unwrap(),
            InventoryEntryKind::File,
            size,
            Some(1),
        )])
        .unwrap()
    }

    #[test]
    fn additive_preflight_needs_no_acknowledgement() {
        let preflight = preflight(
            SyncMode::AddOnly,
            Direction::Upload,
            &inventory("new", 1),
            CaseSensitivity::Sensitive,
            &Inventory::default(),
            CaseSensitivity::Sensitive,
        )
        .unwrap();

        let start = ConfirmedPreflight::from_review(preflight, false).unwrap();

        assert!(start.destructive_confirmation().is_none());
    }

    #[test]
    fn destructive_mirror_requires_and_binds_acknowledgement() {
        let preflight = preflight(
            SyncMode::Mirror,
            Direction::Upload,
            &inventory("changed", 2),
            CaseSensitivity::Sensitive,
            &inventory("changed", 1),
            CaseSensitivity::Sensitive,
        )
        .unwrap();

        assert_eq!(
            ConfirmedPreflight::from_review(preflight.clone(), false),
            Err(StartError::AcknowledgementRequired)
        );
        let start = ConfirmedPreflight::from_review(preflight, true).unwrap();
        assert!(start.destructive_confirmation().is_some());
    }
}
