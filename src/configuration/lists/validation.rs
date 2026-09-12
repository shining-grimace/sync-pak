use super::{ListDocument, relative_path};
use crate::{configuration::SyncMode, validation::validate_name_length};
use std::collections::HashSet;

pub(super) fn validate(document: &ListDocument) -> Result<(), String> {
    if document.format != "syncpak-connection-list" || document.version != 1 {
        return Err("Unsupported connection list format or version.".into());
    }
    let mut ids = HashSet::new();
    for c in &document.connections {
        uuid::Uuid::parse_str(c.id.as_str()).map_err(|_| "Invalid connection identity.")?;
        if !ids.insert(c.id.as_str()) {
            return Err("Duplicate connection identity.".into());
        }
        if c.name.trim().is_empty() {
            return Err("Connection names cannot be empty.".into());
        }
        validate_name_length(&c.name, "Connection name")?;
        relative_path(&c.local_path)?;
        relative_path(&c.remote.path)?;
        let bucket = &c.remote.bucket;
        if bucket.trim().is_empty() || bucket != bucket.trim() || bucket.contains(['/', '\\', '\0'])
        {
            return Err("Invalid bucket name.".into());
        }
        if !c.allow_upload && !c.allow_download {
            return Err("Choose at least one allowed sync direction.".into());
        }
        if c.mode == SyncMode::Archive {
            if c.keep_last_archives.unwrap_or(0) == 0 {
                return Err("Archive retention must be at least 1.".into());
            }
        } else if c.keep_last_archives.is_some() {
            return Err("Only archives can have retention.".into());
        }
    }
    Ok(())
}
