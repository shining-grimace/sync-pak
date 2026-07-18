use aws_sdk_s3::primitives::ByteStream;

use crate::{
    provider_capabilities::{
        ObjectWriteMetadata, ObjectWriter, ProviderResult, SOURCE_MODIFIED_TIME_METADATA_KEY,
    },
    s3_error::provider_error,
    s3_transport::S3Transport,
};

impl ObjectWriter for S3Transport {
    async fn write(&self, bucket: &str, key: &str, contents: &[u8]) -> ProviderResult<()> {
        self.write_with_metadata(bucket, key, contents, &ObjectWriteMetadata::default())
            .await
    }

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
