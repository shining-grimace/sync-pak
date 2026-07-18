use crate::capabilities::CapabilityError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteObject {
    pub key: String,
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
}

pub trait ProviderCapabilities {
    fn list_buckets(&self) -> Result<Vec<String>, CapabilityError>;
    fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<Vec<RemoteObject>, CapabilityError>;
    fn read(&self, bucket: &str, key: &str) -> Result<Vec<u8>, CapabilityError>;
    fn write(&self, bucket: &str, key: &str, contents: &[u8]) -> Result<(), CapabilityError>;
    fn delete(&self, bucket: &str, key: &str) -> Result<(), CapabilityError>;
}
