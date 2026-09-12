use crate::capabilities::list_file::{ListFileCompletion, ListFilePicker};

pub struct PlatformListFilePicker;

#[cfg(not(target_os = "android"))]
impl ListFilePicker for PlatformListFilePicker {
    fn open(&self, completion: ListFileCompletion) -> Result<(), String> {
        std::thread::spawn(move || {
            use std::io::Read;
            let result = rfd::FileDialog::new()
                .set_title("Import connection list")
                .add_filter("Connection list JSON", &["json"])
                .pick_file()
                .map(|path| {
                    let mut bytes = Vec::new();
                    std::fs::File::open(path)
                        .and_then(|file| file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes))
                        .map_err(|_| "The connection list could not be read.".to_owned())?;
                    Ok(bytes)
                })
                .transpose();
            completion(result);
        });
        Ok(())
    }
    fn save(&self, contents: Vec<u8>, completion: ListFileCompletion) -> Result<(), String> {
        std::thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .set_title("Export connection list")
                .add_filter("Connection list JSON", &["json"])
                .set_file_name("connections.json")
                .save_file()
                .map(|path| {
                    super::atomic_write::atomic_write(&path, &contents)
                        .map(|()| vec![])
                        .map_err(|_| "The connection list could not be saved.".to_owned())
                })
                .transpose();
            completion(result);
        });
        Ok(())
    }
}

#[cfg(target_os = "android")]
impl ListFilePicker for PlatformListFilePicker {
    fn open(&self, completion: ListFileCompletion) -> Result<(), String> {
        super::android::list_file::pick(None, completion)
    }
    fn save(&self, contents: Vec<u8>, completion: ListFileCompletion) -> Result<(), String> {
        super::android::list_file::pick(Some(contents), completion)
    }
}
