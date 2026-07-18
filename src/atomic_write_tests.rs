use std::fs;

use uuid::Uuid;

use super::atomic_write;

fn temporary_directory() -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!("sync-pak-atomic-{}", Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    directory
}

#[test]
fn replaces_an_existing_file_only_after_the_new_contents_are_written() {
    let directory = temporary_directory();
    let destination = directory.join("destination.txt");
    fs::write(&destination, "old").unwrap();

    atomic_write(&destination, b"new").unwrap();

    assert_eq!(fs::read_to_string(&destination).unwrap(), "new");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn cleans_up_the_temporary_file_when_replacement_fails() {
    let directory = temporary_directory();
    let destination = directory.join("destination");
    fs::create_dir(&destination).unwrap();

    assert!(atomic_write(&destination, b"new").is_err());
    assert!(destination.is_dir());
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_dir_all(&directory).unwrap();
}
