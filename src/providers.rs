pub mod capabilities;
#[cfg(any(test, feature = "provider-probes"))]
pub mod conformance;
pub(crate) mod error;
pub(crate) mod errors;
#[cfg(any(test, feature = "provider-probes"))]
pub mod multipart_conformance;

#[cfg(feature = "provider-probes")]
pub mod probe;

pub mod verification;

#[cfg(feature = "provider-s3")]
pub mod s3;
