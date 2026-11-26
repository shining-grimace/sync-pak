use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::Path,
};

use uuid::Uuid;

/// Writes a complete replacement beside `path` before renaming it into place.
pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    write_then_publish(path, contents, None, |temporary| {
        fs::rename(temporary, path)
    })
}

pub(crate) fn atomic_create(
    path: &Path,
    contents: &[u8],
    modified: Option<std::time::SystemTime>,
) -> io::Result<()> {
    write_then_publish(path, contents, modified, |temporary| {
        fs::hard_link(temporary, path)?;
        let _ = fs::remove_file(temporary);
        Ok(())
    })
}

fn write_then_publish(
    path: &Path,
    contents: &[u8],
    modified: Option<std::time::SystemTime>,
    publish: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination path has no parent directory",
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination path has no file name",
        )
    })?;
    fs::create_dir_all(directory)?;
    let temporary = directory.join(temporary_name(file_name));
    let write_result = (|| {
        let mut file = fs::File::options()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        if let Some(modified) = modified {
            let _ = file.set_times(fs::FileTimes::new().set_modified(modified));
        }
        file.sync_all()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    publish(&temporary).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

fn temporary_name(file_name: &std::ffi::OsStr) -> OsString {
    let mut name = OsString::from(".");
    name.push(file_name);
    name.push(format!(".sync-pak-{}.tmp", Uuid::new_v4()));
    name
}

#[cfg(test)]
mod tests;
