#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityError {
    Busy,
    TimedOut,
    UnlockTimedOut,
    Cancelled,
    ServiceMissing,
    NotConfigured,
    Locked,
    AccessDenied,
    InvalidReference,
    NotFound,
    Unsupported,
    UnsupportedPath,
    Unavailable,
    Unexpected,
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Busy => "Another operating system request is already in progress.",
            Self::TimedOut => "Protected credential storage did not respond in time.",
            Self::UnlockTimedOut => "Credential storage was not unlocked in time.",
            Self::Cancelled => "Credential access was cancelled.",
            Self::ServiceMissing => "No desktop credential service is available.",
            Self::NotConfigured => "No default credential collection is configured.",
            Self::Locked => "Credential storage is still locked.",
            Self::AccessDenied => "Access to credential storage was denied.",
            Self::InvalidReference => "The protected credential reference is invalid.",
            Self::NotFound => "The protected credential was not found.",
            Self::Unsupported => "This capability is not implemented on this platform yet.",
            Self::UnsupportedPath => "The selected folder cannot be represented safely as UTF-8.",
            Self::Unavailable => "The operating system facility is locked or unavailable.",
            Self::Unexpected => "The operating system could not complete the request.",
        })
    }
}

impl std::error::Error for CapabilityError {}
