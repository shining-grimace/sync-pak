use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    provider_conformance::{ConformanceError, verify_bucket_listing, verify_object_lifecycle},
    provider_multipart_conformance::verify_multipart_lifecycle,
    s3_transport::S3Transport,
};

use crate::provider_probe_config::ProbeConfig;

#[derive(Debug, Eq, PartialEq)]
pub enum ProbeError {
    MissingSetting(&'static str),
    InvalidSetting(&'static str),
    Connection,
    Conformance(ConformanceError),
    Clock,
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MissingSetting(name) => {
                return write!(formatter, "Missing or empty setting: {name}.");
            }
            Self::InvalidSetting(name) => return write!(formatter, "Invalid setting: {name}."),
            Self::Connection => "The provider probe could not create a provider connection.",
            Self::Conformance(error) => return write!(formatter, "The provider probe {error}"),
            Self::Clock => "The provider probe could not create a unique test object name.",
        })
    }
}

impl std::error::Error for ProbeError {}

/// Runs the shared behavior suite against a disposable prefix using environment credentials.
pub async fn run_from_environment() -> Result<(), ProbeError> {
    let config = ProbeConfig::from_environment()?;
    let transport = S3Transport::connect(&config.provider(), config.credentials())
        .await
        .map_err(|_| ProbeError::Connection)?;
    if config.check_bucket_listing {
        verify_bucket_listing(&transport, &config.bucket)
            .await
            .map_err(ProbeError::Conformance)?;
    }
    verify_object_lifecycle(
        &transport,
        &config.bucket,
        &config.prefix,
        &test_key(&config.prefix, "object")?,
    )
    .await
    .map_err(ProbeError::Conformance)?;
    verify_multipart_lifecycle(
        &transport,
        &config.bucket,
        &test_key(&config.prefix, "multipart")?,
    )
    .await
    .map_err(ProbeError::Conformance)
}

fn test_key(prefix: &str, kind: &str) -> Result<String, ProbeError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProbeError::Clock)?
        .as_millis();
    Ok(format!(
        "{prefix}/syncpak-conformance-{kind}-{milliseconds}-{}.txt",
        std::process::id()
    ))
}

#[cfg(test)]
#[path = "provider_probe_tests.rs"]
mod tests;
