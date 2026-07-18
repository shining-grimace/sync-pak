//! Provider-neutral contracts for remote object storage.
//!
//! Providers implement only the traits their credentials and API support. This keeps an
//! unavailable optional feature, such as multipart upload, out of the normal object API.

use std::future::Future;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectMetadata {
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
    pub content_type: Option<String>,
    pub entity_tag: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteObject {
    pub key: String,
    pub metadata: ObjectMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultipartUpload {
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UploadedPart {
    pub part_number: u32,
    pub entity_tag: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultipartUploadRequest {
    pub bucket: String,
    pub key: String,
    pub content_type: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderError {
    Authentication,
    InvalidRequest,
    NotFound,
    PermissionDenied,
    Unavailable,
    Unsupported,
    Unexpected,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Authentication => "The provider rejected the saved credentials.",
            Self::InvalidRequest => "The provider request is not valid.",
            Self::NotFound => "The requested provider resource was not found.",
            Self::PermissionDenied => "The provider did not allow this operation.",
            Self::Unavailable => "The provider could not be reached.",
            Self::Unsupported => "The provider does not support this operation.",
            Self::Unexpected => "The provider could not complete the operation.",
        })
    }
}

impl std::error::Error for ProviderError {}

pub type ProviderResult<T> = Result<T, ProviderError>;

pub trait BucketLister {
    fn list_buckets(&self) -> impl Future<Output = ProviderResult<Vec<String>>> + Send;
}

pub trait ObjectLister {
    fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> impl Future<Output = ProviderResult<Vec<RemoteObject>>> + Send;
}

pub trait ObjectReader {
    fn read(&self, bucket: &str, key: &str)
    -> impl Future<Output = ProviderResult<Vec<u8>>> + Send;
}

pub trait ObjectWriter {
    fn write(
        &self,
        bucket: &str,
        key: &str,
        contents: &[u8],
    ) -> impl Future<Output = ProviderResult<()>> + Send;
}

pub trait ObjectMetadataReader {
    fn metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> impl Future<Output = ProviderResult<ObjectMetadata>> + Send;
}

pub trait ObjectDeleter {
    fn delete(&self, bucket: &str, key: &str) -> impl Future<Output = ProviderResult<()>> + Send;
}

pub trait MultipartUploader {
    fn begin_multipart_upload(
        &self,
        request: &MultipartUploadRequest,
    ) -> impl Future<Output = ProviderResult<MultipartUpload>> + Send;

    fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload: &MultipartUpload,
        part_number: u32,
        contents: &[u8],
    ) -> impl Future<Output = ProviderResult<UploadedPart>> + Send;

    fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload: &MultipartUpload,
        parts: &[UploadedPart],
    ) -> impl Future<Output = ProviderResult<()>> + Send;

    fn abort_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload: &MultipartUpload,
    ) -> impl Future<Output = ProviderResult<()>> + Send;
}

#[cfg(test)]
mod tests {
    use super::{ObjectMetadata, ProviderError, RemoteObject};

    #[test]
    fn object_metadata_preserves_provider_values() {
        let object = RemoteObject {
            key: "photos/é.png".to_owned(),
            metadata: ObjectMetadata {
                byte_size: 42,
                modified_unix_seconds: Some(1_700_000_000),
                content_type: Some("image/png".to_owned()),
                entity_tag: Some("abc123".to_owned()),
            },
        };

        assert_eq!(object.metadata.byte_size, 42);
        assert_eq!(object.metadata.entity_tag.as_deref(), Some("abc123"));
    }

    #[test]
    fn provider_errors_have_redaction_safe_messages() {
        assert_eq!(
            ProviderError::Authentication.to_string(),
            "The provider rejected the saved credentials."
        );
    }
}
