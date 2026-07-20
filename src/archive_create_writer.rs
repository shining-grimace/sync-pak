use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    archive_create::ArchiveCreateError,
    cancellation::CancellationToken,
    inventory::{Inventory, InventoryEntryKind},
    transfer_paths::LocalTransferRoot,
};

pub(crate) fn write_archive(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    temporary: &Path,
    cancellation: &CancellationToken,
) -> Result<(), ArchiveCreateError> {
    let file = fs::File::options()
        .create_new(true)
        .write(true)
        .open(temporary)
        .map_err(ArchiveCreateError::Local)?;
    let mut archive = ZipWriter::new(file);
    for entry in inventory.entries() {
        cancellation
            .check()
            .map_err(|_| ArchiveCreateError::Cancelled)?;
        let name = entry.path.as_str();
        match &entry.kind {
            InventoryEntryKind::Directory => archive
                .add_directory(format!("{name}/"), options(0o40755))
                .map_err(ArchiveCreateError::Zip)?,
            InventoryEntryKind::File => write_file(&mut archive, source_root, entry, cancellation)?,
            InventoryEntryKind::Symlink { target } => {
                archive
                    .start_file(name, options(0o120777))
                    .map_err(ArchiveCreateError::Zip)?;
                archive
                    .write_all(target.as_bytes())
                    .map_err(ArchiveCreateError::Local)?;
            }
        }
    }
    archive
        .finish()
        .map_err(ArchiveCreateError::Zip)?
        .sync_all()
        .map_err(ArchiveCreateError::Local)
}

fn write_file(
    archive: &mut ZipWriter<fs::File>,
    source_root: &LocalTransferRoot,
    entry: &crate::inventory::InventoryEntry,
    cancellation: &CancellationToken,
) -> Result<(), ArchiveCreateError> {
    archive
        .start_file(entry.path.as_str(), options(0o100644))
        .map_err(ArchiveCreateError::Zip)?;
    let mut source =
        fs::File::open(source_root.resolve(&entry.path)).map_err(ArchiveCreateError::Local)?;
    let mut buffer = [0; 64 * 1024];
    loop {
        cancellation
            .check()
            .map_err(|_| ArchiveCreateError::Cancelled)?;
        let count = source
            .read(&mut buffer)
            .map_err(ArchiveCreateError::Local)?;
        if count == 0 {
            return Ok(());
        }
        archive
            .write_all(&buffer[..count])
            .map_err(ArchiveCreateError::Local)?;
    }
}

fn options(permissions: u32) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(permissions)
}
