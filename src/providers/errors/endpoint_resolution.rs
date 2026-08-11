use std::{error::Error, io};

#[derive(Debug)]
pub(crate) struct EndpointResolutionError {
    source: io::Error,
}

impl EndpointResolutionError {
    pub(crate) fn new(source: io::Error) -> Self {
        Self { source }
    }
}

impl std::fmt::Display for EndpointResolutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("provider endpoint resolution failed")
    }
}

impl Error for EndpointResolutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
