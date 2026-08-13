use crate::inventory::InventoryEntryKind;

pub fn parse_size(value: String) -> rusqlite::Result<u64> {
    value.parse().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

pub fn encode_kind(kind: &InventoryEntryKind) -> i64 {
    match kind {
        InventoryEntryKind::File => 0,
        InventoryEntryKind::Directory => 1,
        InventoryEntryKind::Symlink { .. } => 2,
    }
}

pub fn decode_kind(value: i64) -> rusqlite::Result<InventoryEntryKind> {
    match value {
        0 => Ok(InventoryEntryKind::File),
        1 => Ok(InventoryEntryKind::Directory),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| duration.as_secs().try_into().ok())
        .unwrap_or_default()
}
