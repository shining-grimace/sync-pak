use std::time::{SystemTime, UNIX_EPOCH};

use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    primitives::ByteStream,
};

use crate::provider_probe_config::{ProbeConfig, ProviderKind};

const CONTENT: &[u8] = b"SyncPak provider feasibility probe\n";

#[derive(Debug, Eq, PartialEq)]
pub enum ProbeError {
    MissingSetting(&'static str),
    InvalidSetting(&'static str),
    List,
    Upload,
    Download,
    Verification,
    Cleanup,
    Clock,
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSetting(name) => write!(formatter, "Missing or empty setting: {name}."),
            Self::InvalidSetting(name) => write!(formatter, "Invalid setting: {name}."),
            Self::List => {
                formatter.write_str("The provider probe could not list the isolated test prefix.")
            }
            Self::Upload => {
                formatter.write_str("The provider probe could not upload its test object.")
            }
            Self::Download => {
                formatter.write_str("The provider probe could not download its test object.")
            }
            Self::Verification => {
                formatter.write_str("The provider probe downloaded unexpected test content.")
            }
            Self::Cleanup => {
                formatter.write_str("The provider probe could not remove its test object.")
            }
            Self::Clock => formatter
                .write_str("The provider probe could not create a unique test object name."),
        }
    }
}

impl std::error::Error for ProbeError {}

/// Runs list, upload, download, verification, and deletion against an isolated test prefix.
pub async fn run_from_environment() -> Result<(), ProbeError> {
    let config = ProbeConfig::from_environment()?;
    let client = client(&config).await;
    let key = test_key(&config.prefix)?;

    client
        .list_objects_v2()
        .bucket(&config.bucket)
        .prefix(&config.prefix)
        .max_keys(1)
        .send()
        .await
        .map_err(|_| ProbeError::List)?;
    client
        .put_object()
        .bucket(&config.bucket)
        .key(&key)
        .body(ByteStream::from_static(CONTENT))
        .send()
        .await
        .map_err(|_| ProbeError::Upload)?;

    let verification = verify_download(&client, &config.bucket, &key).await;
    let cleanup = client
        .delete_object()
        .bucket(&config.bucket)
        .key(&key)
        .send()
        .await
        .map(|_| ())
        .map_err(|_| ProbeError::Cleanup);
    match (verification, cleanup) {
        (_, Err(error)) => Err(error),
        (result, Ok(())) => result,
    }
}

async fn client(config: &ProbeConfig) -> Client {
    let credentials = Credentials::new(
        &config.access_key_id,
        &config.secret_access_key,
        None,
        None,
        "sync-pak-provider-probe",
    );
    let mut loader = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(config.region.clone()));
    if let Some(endpoint) = &config.endpoint {
        loader = loader.endpoint_url(endpoint);
    }
    let shared = loader.load().await;
    let service = aws_sdk_s3::config::Builder::from(&shared)
        .force_path_style(!matches!(config.kind, ProviderKind::AwsS3))
        .build();
    Client::from_conf(service)
}

async fn verify_download(client: &Client, bucket: &str, key: &str) -> Result<(), ProbeError> {
    let object = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|_| ProbeError::Download)?;
    let contents = object
        .body
        .collect()
        .await
        .map_err(|_| ProbeError::Download)?;
    (contents.into_bytes().as_ref() == CONTENT)
        .then_some(())
        .ok_or(ProbeError::Verification)
}

fn test_key(prefix: &str) -> Result<String, ProbeError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProbeError::Clock)?
        .as_millis();
    Ok(format!(
        "{prefix}/syncpak-feasibility-{milliseconds}-{}.txt",
        std::process::id()
    ))
}

#[cfg(test)]
#[path = "provider_probe_tests.rs"]
mod tests;
