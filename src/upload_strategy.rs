pub const MULTIPART_THRESHOLD_BYTES: u64 = 8 * 1024 * 1024;
pub const MULTIPART_PART_SIZE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UploadStrategy {
    SinglePart,
    Multipart { part_size: usize },
}

pub fn select_upload_strategy(byte_size: u64) -> UploadStrategy {
    if byte_size < MULTIPART_THRESHOLD_BYTES {
        UploadStrategy::SinglePart
    } else {
        UploadStrategy::Multipart {
            part_size: MULTIPART_PART_SIZE_BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MULTIPART_PART_SIZE_BYTES, MULTIPART_THRESHOLD_BYTES, UploadStrategy,
        select_upload_strategy,
    };

    #[test]
    fn selects_multipart_at_the_threshold_and_single_part_below_it() {
        assert_eq!(
            select_upload_strategy(MULTIPART_THRESHOLD_BYTES - 1),
            UploadStrategy::SinglePart
        );
        assert_eq!(
            select_upload_strategy(MULTIPART_THRESHOLD_BYTES),
            UploadStrategy::Multipart {
                part_size: MULTIPART_PART_SIZE_BYTES,
            }
        );
    }
}
