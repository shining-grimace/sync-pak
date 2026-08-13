use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region, RequestChecksumCalculation},
};

use crate::{
    configuration::{ProviderConfig, ProviderCredentials},
    providers::capabilities::{
        ObjectDeleter, ObjectLister, ObjectMetadata, ObjectReader, ProviderError, ProviderResult,
        ReadObject, RemoteObject,
    },
    providers::s3::error::provider_error,
    providers::s3::settings::S3Settings,
};

use crate::providers::capabilities::ObjectMetadataReader;

pub struct S3Transport {
    pub(crate) client: Client,
}

impl S3Transport {
    pub async fn connect(
        provider: &ProviderConfig,
        credentials: ProviderCredentials,
    ) -> ProviderResult<Self> {
        let settings = S3Settings::from_provider(provider)?;
        let credentials = Credentials::new(
            credentials.access_key_id,
            credentials.secret_access_key,
            credentials.session_token,
            None,
            "sync-pak",
        );
        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(settings.region));
        #[cfg(target_os = "android")]
        {
            loader = loader.http_client(crate::platform::android::s3::http_client::build()?);
        }
        if let Some(endpoint) = settings.endpoint {
            loader = loader.endpoint_url(endpoint);
        }
        let shared = loader.load().await;
        let mut config =
            aws_sdk_s3::config::Builder::from(&shared).force_path_style(settings.force_path_style);
        if settings.request_checksums_when_required {
            config = config.request_checksum_calculation(RequestChecksumCalculation::WhenRequired);
        }
        Ok(Self {
            client: Client::from_conf(config.build()),
        })
    }
}

impl ObjectLister for S3Transport {
    async fn list_objects(&self, bucket: &str, prefix: &str) -> ProviderResult<Vec<RemoteObject>> {
        let mut objects = Vec::new();
        let mut continuation_token = None;
        loop {
            let mut request = self.client.list_objects_v2().bucket(bucket).prefix(prefix);
            if let Some(token) = continuation_token.as_deref() {
                request = request.continuation_token(token);
            }
            let response = request.send().await.map_err(provider_error)?;
            objects.extend(
                response
                    .contents()
                    .iter()
                    .map(remote_object)
                    .collect::<ProviderResult<Vec<_>>>()?,
            );
            if !response.is_truncated().unwrap_or(false) {
                return Ok(objects);
            }
            continuation_token = Some(
                response
                    .next_continuation_token()
                    .map(ToOwned::to_owned)
                    .ok_or(ProviderError::Unexpected)?,
            );
        }
    }
}

impl ObjectReader for S3Transport {
    async fn read(&self, bucket: &str, key: &str) -> ProviderResult<Vec<u8>> {
        Ok(self.read_with_metadata(bucket, key).await?.contents)
    }

    async fn read_with_metadata(&self, bucket: &str, key: &str) -> ProviderResult<ReadObject> {
        let object = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(provider_error)?;
        let metadata = object_metadata(
            object.content_length(),
            object.last_modified().map(|value| value.secs()),
            source_modified_time(object.metadata()),
            object.content_type(),
            object.e_tag(),
        )?;
        let contents = object
            .body
            .collect()
            .await
            .map(|contents| contents.into_bytes().to_vec())
            .map_err(|_| ProviderError::Unavailable)?;
        Ok(ReadObject {
            contents,
            metadata: Some(metadata),
        })
    }
}

impl ObjectMetadataReader for S3Transport {
    async fn metadata(&self, bucket: &str, key: &str) -> ProviderResult<ObjectMetadata> {
        let object = self
            .client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(provider_error)?;
        object_metadata(
            object.content_length(),
            object.last_modified().map(|value| value.secs()),
            source_modified_time(object.metadata()),
            object.content_type(),
            object.e_tag(),
        )
    }
}

impl ObjectDeleter for S3Transport {
    async fn delete(&self, bucket: &str, key: &str) -> ProviderResult<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(provider_error)
    }
}

fn remote_object(object: &aws_sdk_s3::types::Object) -> ProviderResult<RemoteObject> {
    Ok(RemoteObject {
        key: object
            .key()
            .map(ToOwned::to_owned)
            .ok_or(ProviderError::Unexpected)?,
        metadata: object_metadata(
            object.size(),
            object.last_modified().map(|value| value.secs()),
            None,
            None,
            object.e_tag(),
        )?,
    })
}

fn object_metadata(
    byte_size: Option<i64>,
    modified_unix_seconds: Option<i64>,
    source_modified_unix_seconds: Option<i64>,
    content_type: Option<&str>,
    entity_tag: Option<&str>,
) -> ProviderResult<ObjectMetadata> {
    Ok(ObjectMetadata {
        byte_size: u64::try_from(byte_size.ok_or(ProviderError::Unexpected)?)
            .map_err(|_| ProviderError::Unexpected)?,
        modified_unix_seconds,
        source_modified_unix_seconds,
        content_type: content_type.map(ToOwned::to_owned),
        entity_tag: entity_tag.map(ToOwned::to_owned),
    })
}

fn source_modified_time(
    metadata: Option<&std::collections::HashMap<String, String>>,
) -> Option<i64> {
    metadata
        .and_then(|metadata| {
            metadata.get(crate::providers::capabilities::SOURCE_MODIFIED_TIME_METADATA_KEY)
        })
        .and_then(|value| value.parse().ok())
}
