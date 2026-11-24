use futures_util::FutureExt;

use aws_sdk_s3::primitives::ByteStream;

use crate::{
    providers::capabilities::{
        ObjectPrefixCreator, ObjectWriteMetadata, ObjectWriter, ProviderResult,
        SOURCE_MODIFIED_TIME_METADATA_KEY,
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
        request
            .send()
            .boxed()
            .await
            .map(|_| ())
            .map_err(provider_error)
    }
}

impl ObjectPrefixCreator for S3Transport {
    async fn create_prefix(&self, bucket: &str, prefix: &str) -> ProviderResult<()> {
        match self
            .client
            .put_object()
            .bucket(bucket)
            .key(prefix)
            .body(ByteStream::from_static(b""))
            .if_none_match("*")
            .send()
            .boxed()
            .await
        {
            Ok(_) => Ok(()),
            Err(error)
                if error
                    .raw_response()
                    .is_some_and(|response| response.status().as_u16() == 412) =>
            {
                Ok(())
            }
            Err(error) => Err(provider_error(error)),
        }
    }
}
