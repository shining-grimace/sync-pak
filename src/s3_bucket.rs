use crate::{
    provider_capabilities::{BucketAccessChecker, BucketLister, ProviderResult},
    s3_error::provider_error,
    s3_transport::S3Transport,
};

impl BucketAccessChecker for S3Transport {
    async fn check_bucket_access(&self, bucket: &str) -> ProviderResult<()> {
        self.client
            .list_objects_v2()
            .bucket(bucket)
            .max_keys(1)
            .send()
            .await
            .map(|_| ())
            .map_err(provider_error)
    }
}

impl BucketLister for S3Transport {
    async fn list_buckets(&self) -> ProviderResult<Vec<String>> {
        let response = self
            .client
            .list_buckets()
            .send()
            .await
            .map_err(provider_error)?;
        Ok(response
            .buckets()
            .iter()
            .filter_map(|bucket| bucket.name().map(ToOwned::to_owned))
            .collect())
    }
}
