use aws_sdk_s3::primitives::ByteStream;

use crate::{
    providers::capabilities::{
        ObjectWriteMetadata, ObjectWriter, ProviderResult, SOURCE_MODIFIED_TIME_METADATA_KEY,
    },
    providers::s3::error::provider_error,
    providers::s3::transport::S3Transport,
};

impl ObjectWriter for S3Transport {
    async fn write_with_metadata(
        &self,
        bucket: &str,
        key: &str,
        contents: &[u8],
        metadata: &ObjectWriteMetadata,
    ) -> ProviderResult<()> {
        let mut request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(contents.to_vec()));
        if let Some(modified) = metadata.source_modified_unix_seconds {
            request = request.metadata(SOURCE_MODIFIED_TIME_METADATA_KEY, modified.to_string());
        }
        request.send().await.map(|_| ()).map_err(provider_error)
    }
}
