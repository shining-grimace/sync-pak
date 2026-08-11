use std::hash::{Hash, Hasher};

use crate::inventory::{Inventory, InventoryEntryKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InventoryFingerprint(u64);

pub fn fingerprint(inventory: &Inventory) -> InventoryFingerprint {
    let mut hasher = std::hash::DefaultHasher::new();
    for entry in inventory.entries() {
        entry.path.as_str().hash(&mut hasher);
        match &entry.kind {
            InventoryEntryKind::File => 0_u8.hash(&mut hasher),
            InventoryEntryKind::Directory => 1_u8.hash(&mut hasher),
            InventoryEntryKind::Symlink { target } => {
                2_u8.hash(&mut hasher);
                target.hash(&mut hasher);
            }
        }
        entry.byte_size.hash(&mut hasher);
        entry.modified_unix_seconds.hash(&mut hasher);
    }
    InventoryFingerprint(hasher.finish())
}
