use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    error::{ProvideErrorMetadata, SdkError},
    primitives::ByteStream,
};

use crate::{
    configuration::{ProviderConfig, ProviderCredentials, ProviderKind},
    provider_capabilities::{
        BucketLister, ObjectDeleter, ObjectLister, ObjectMetadata, ObjectMetadataReader,
        ObjectReader, ObjectWriter, ProviderError, ProviderResult, RemoteObject,
    },
};

pub struct S3Transport {
    client: Client,
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
        if let Some(endpoint) = settings.endpoint {
            loader = loader.endpoint_url(endpoint);
        }
        let shared = loader.load().await;
        let config = aws_sdk_s3::config::Builder::from(&shared)
            .force_path_style(settings.force_path_style)
            .build();
        Ok(Self {
            client: Client::from_conf(config),
        })
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
        let object = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(provider_error)?;
        object
            .body
            .collect()
            .await
            .map(|contents| contents.into_bytes().to_vec())
            .map_err(|_| ProviderError::Unavailable)
    }
}

impl ObjectWriter for S3Transport {
    async fn write(&self, bucket: &str, key: &str, contents: &[u8]) -> ProviderResult<()> {
        self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(contents.to_vec()))
            .send()
            .await
            .map(|_| ())
            .map_err(provider_error)
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

struct S3Settings {
    endpoint: Option<String>,
    force_path_style: bool,
    region: String,
}

impl S3Settings {
    fn from_provider(provider: &ProviderConfig) -> ProviderResult<Self> {
        let endpoint = match provider.kind {
            ProviderKind::AwsS3 => provider.options.endpoint.clone(),
            ProviderKind::CloudflareR2 => Some(format!(
                "https://{}.r2.cloudflarestorage.com",
                provider
                    .options
                    .account_id
                    .as_deref()
                    .ok_or(ProviderError::InvalidRequest)?
            )),
            ProviderKind::BackblazeB2 => Some(
                provider
                    .options
                    .endpoint
                    .clone()
                    .ok_or(ProviderError::InvalidRequest)?,
            ),
        };
        let region = match provider.kind {
            ProviderKind::CloudflareR2 => "auto".to_owned(),
            ProviderKind::AwsS3 | ProviderKind::BackblazeB2 => provider
                .options
                .region
                .clone()
                .ok_or(ProviderError::InvalidRequest)?,
        };
        Ok(Self {
            endpoint,
            force_path_style: !matches!(provider.kind, ProviderKind::AwsS3),
            region,
        })
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
            object.e_tag(),
        )?,
    })
}

fn object_metadata(
    byte_size: Option<i64>,
    modified_unix_seconds: Option<i64>,
    content_type: Option<&str>,
    entity_tag: Option<&str>,
) -> ProviderResult<ObjectMetadata> {
    Ok(ObjectMetadata {
        byte_size: u64::try_from(byte_size.ok_or(ProviderError::Unexpected)?)
            .map_err(|_| ProviderError::Unexpected)?,
        modified_unix_seconds,
        content_type: content_type.map(ToOwned::to_owned),
        entity_tag: entity_tag.map(ToOwned::to_owned),
    })
}

fn provider_error<E: ProvideErrorMetadata>(error: SdkError<E>) -> ProviderError {
    match error.as_service_error().and_then(|value| value.code()) {
        Some("AccessDenied") => ProviderError::PermissionDenied,
        Some("InvalidAccessKeyId" | "InvalidToken" | "SignatureDoesNotMatch") => {
            ProviderError::Authentication
        }
        Some("NoSuchBucket" | "NoSuchKey" | "NotFound") => ProviderError::NotFound,
        _ => ProviderError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::S3Settings;
    use crate::configuration::{
        CredentialReference, ProviderConfig, ProviderId, ProviderKind, ProviderOptions,
    };

    #[test]
    fn r2_uses_its_account_endpoint_and_auto_region() {
        let settings = S3Settings::from_provider(&provider(
            ProviderKind::CloudflareR2,
            Some("account"),
            None,
            None,
        ))
        .unwrap();

        assert_eq!(
            settings.endpoint.as_deref(),
            Some("https://account.r2.cloudflarestorage.com")
        );
        assert_eq!(settings.region, "auto");
        assert!(settings.force_path_style);
    }

    #[test]
    fn b2_requires_an_explicit_regional_endpoint() {
        assert!(
            S3Settings::from_provider(&provider(
                ProviderKind::BackblazeB2,
                None,
                None,
                Some("us-west-001")
            ))
            .is_err()
        );
    }

    fn provider(
        kind: ProviderKind,
        account_id: Option<&str>,
        endpoint: Option<&str>,
        region: Option<&str>,
    ) -> ProviderConfig {
        let id = ProviderId::new();
        ProviderConfig {
            credential_reference: CredentialReference {
                provider_id: id.clone(),
            },
            id,
            name: "Test".to_owned(),
            kind,
            options: ProviderOptions {
                account_id: account_id.map(ToOwned::to_owned),
                default_bucket: None,
                endpoint: endpoint.map(ToOwned::to_owned),
                region: region.map(ToOwned::to_owned),
            },
        }
    }
}
