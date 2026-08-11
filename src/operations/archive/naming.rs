use std::{error::Error, fmt};

use unicode_normalization::UnicodeNormalization;

const MAX_FILENAME_BYTES: usize = 200;

/// Builds a portable archive filename from a UTC timestamp and user-facing connection name.
pub fn archive_filename(
    timestamp: &str,
    connection_name: &str,
) -> Result<String, ArchiveNameError> {
    if !is_utc_timestamp(timestamp) {
        return Err(ArchiveNameError::InvalidTimestamp(timestamp.into()));
    }
    let suffix = format!("{timestamp} .zip");
    let available_name_bytes = MAX_FILENAME_BYTES - suffix.len();
    let sanitized_name = sanitize_connection_name(connection_name);
    let name = truncate_utf8(&sanitized_name, available_name_bytes);
    Ok(format!("{timestamp} {name}.zip"))
}

fn is_utc_timestamp(timestamp: &str) -> bool {
    timestamp.len() == 16
        && timestamp.as_bytes()[8] == b'-'
        && timestamp.as_bytes()[15] == b'Z'
        && timestamp
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 8 | 15) || byte.is_ascii_digit())
}

fn sanitize_connection_name(connection_name: &str) -> String {
    let name = connection_name
        .nfc()
        .map(|character| match character {
            '\u{0000}'..='\u{001f}' | '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            character => character,
        })
        .collect::<String>();
    let name = name.trim_end_matches([' ', '.']);
    (!name.is_empty())
        .then_some(name)
        .unwrap_or("Archive")
        .into()
}

fn truncate_utf8(value: &str, maximum_bytes: usize) -> &str {
    let mut end = 0;
    for (index, character) in value.char_indices() {
        let next = index + character.len_utf8();
        if next > maximum_bytes {
            break;
        }
        end = next;
    }
    &value[..end]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveNameError {
    InvalidTimestamp(String),
}

impl fmt::Display for ArchiveNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimestamp(timestamp) => {
                write!(formatter, "invalid archive timestamp: {timestamp}")
            }
        }
    }
}

impl Error for ArchiveNameError {}

#[cfg(test)]
mod tests {
    use super::{MAX_FILENAME_BYTES, archive_filename};

    #[test]
    fn normalizes_and_sanitizes_portable_archive_names() {
        let filename = archive_filename("20260720-123456Z", "Cafe\u{301}/backup. ").unwrap();

        assert_eq!(filename, "20260720-123456Z Café_backup.zip");
    }

    #[test]
    fn replaces_an_empty_sanitized_name_and_truncates_on_a_utf8_boundary() {
        let fallback = archive_filename("20260720-123456Z", "... ").unwrap();
        let long = archive_filename("20260720-123456Z", &"é".repeat(200)).unwrap();

        assert_eq!(fallback, "20260720-123456Z Archive.zip");
        assert!(long.len() <= MAX_FILENAME_BYTES);
        assert!(long.is_char_boundary(long.len()));
    }
}
