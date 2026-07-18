use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::Path,
};

use uuid::Uuid;

/// Writes a complete replacement beside `path` before renaming it into place.
pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
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
        file.sync_all()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

fn temporary_name(file_name: &std::ffi::OsStr) -> OsString {
    let mut name = OsString::from(".");
    name.push(file_name);
    name.push(format!(".{}.tmp", Uuid::new_v4()));
    name
}

#[cfg(test)]
#[path = "atomic_write_tests.rs"]
mod tests;
