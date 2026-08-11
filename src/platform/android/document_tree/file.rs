use std::{
    fs::File,
    io::{self, Write},
    os::fd::FromRawFd,
};

use crate::platform::android::document_tree::model::status_error;

pub fn open_file(descriptor: i32) -> io::Result<File> {
    if descriptor < 0 {
        return Err(status_error(descriptor));
    }
    // SAFETY: ParcelFileDescriptor.detachFd transfers ownership of this descriptor to Rust.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub fn finish_write(file: &mut File) -> io::Result<()> {
    file.flush()?;
    match file.sync_all() {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::InvalidInput | io::ErrorKind::Unsupported
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}
