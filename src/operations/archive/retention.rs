use std::{error::Error, fmt};

use crate::configuration::ConnectionId;

/// A successfully stored archive, identified independently of its editable connection name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveRecord {
    pub connection_id: ConnectionId,
    pub location: String,
    pub created_at_utc: String,
}

/// Selects older archives that may be pruned after `new_archive` has been stored.
pub fn prune_after_success(
    existing: &[ArchiveRecord],
    new_archive: &ArchiveRecord,
    keep_last: u32,
) -> Result<Vec<ArchiveRecord>, ArchiveRetentionError> {
    if keep_last == 0 {
        return Err(ArchiveRetentionError::KeepLastZero);
    }
    let mut owned = existing
        .iter()
        .filter(|archive| archive.connection_id == new_archive.connection_id)
        .cloned()
        .collect::<Vec<_>>();
    if !owned
        .iter()
        .any(|archive| archive.location == new_archive.location)
    {
        owned.push(new_archive.clone());
    }
    owned.sort_by(|left, right| right.created_at_utc.cmp(&left.created_at_utc));
    Ok(owned.into_iter().skip(keep_last as usize).collect())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveRetentionError {
    KeepLastZero,
}

impl fmt::Display for ArchiveRetentionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("archive retention must keep at least one archive")
    }
}

impl Error for ArchiveRetentionError {}

#[cfg(test)]
mod tests {
    use crate::configuration::ConnectionId;

    use super::{ArchiveRecord, ArchiveRetentionError, prune_after_success};

    fn archive(
        connection_id: &ConnectionId,
        location: &str,
        created_at_utc: &str,
    ) -> ArchiveRecord {
        ArchiveRecord {
            connection_id: connection_id.clone(),
            location: location.into(),
            created_at_utc: created_at_utc.into(),
        }
    }

    #[test]
    fn prunes_only_owned_archives_after_the_new_archive_is_present() {
        let connection = ConnectionId::new();
        let other_connection = ConnectionId::new();
        let new_archive = archive(&connection, "new.zip", "20260720-120000Z");
        let existing = [
            archive(&connection, "old.zip", "20260719-120000Z"),
            archive(&other_connection, "other.zip", "20260718-120000Z"),
        ];

        let prune = prune_after_success(&existing, &new_archive, 1).unwrap();

        assert_eq!(prune, [archive(&connection, "old.zip", "20260719-120000Z")]);
    }

    #[test]
    fn rejects_a_retention_policy_that_would_remove_every_archive() {
        let connection = ConnectionId::new();
        let archive = archive(&connection, "new.zip", "20260720-120000Z");

        assert_eq!(
            prune_after_success(&[], &archive, 0),
            Err(ArchiveRetentionError::KeepLastZero)
        );
    }
}
