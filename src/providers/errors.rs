pub(crate) mod connectivity_failure;
#[cfg(all(feature = "provider-s3", any(target_os = "android", test)))]
pub(crate) mod endpoint_resolution;
pub(crate) mod safe_transport_detail;
#[cfg(all(feature = "provider-s3", any(target_os = "android", test)))]
pub(crate) mod secure_connection;
