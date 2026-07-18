use aws_sdk_s3::error::{ProvideErrorMetadata, SdkError};

use crate::provider_capabilities::ProviderError;

pub(crate) fn provider_error<E: ProvideErrorMetadata>(error: SdkError<E>) -> ProviderError {
    if let Some(code) = error.as_service_error().and_then(|value| value.code()) {
        return provider_error_code(code);
    }
    match error {
        SdkError::TimeoutError(_) | SdkError::DispatchFailure(_) => ProviderError::Unavailable,
        SdkError::ConstructionFailure(_) => ProviderError::InvalidRequest,
        SdkError::ResponseError(_) | SdkError::ServiceError(_) => ProviderError::Unexpected,
        _ => ProviderError::Unexpected,
    }
}

fn provider_error_code(code: &str) -> ProviderError {
    match code {
        "AccessDenied" | "NotEntitled" | "ObjectLockedByBucketPolicy" => {
            ProviderError::PermissionDenied
        }
        "Unauthorized"
        | "InvalidAccessKeyId"
        | "InvalidToken"
        | "ExpiredToken"
        | "SignatureDoesNotMatch" => ProviderError::Authentication,
        "NoSuchBucket" | "NoSuchKey" | "NoSuchUpload" | "NotFound" => ProviderError::NotFound,
        "InternalError" | "ServiceUnavailable" | "SlowDown" | "TooManyRequests"
        | "RequestTimeout" => ProviderError::Unavailable,
        "AuthorizationHeaderMalformed"
        | "BadDigest"
        | "EntityTooLarge"
        | "EntityTooSmall"
        | "ExpiredRequest"
        | "IncompleteBody"
        | "InvalidArgument"
        | "InvalidBucketName"
        | "InvalidDigest"
        | "InvalidObjectName"
        | "InvalidPart"
        | "InvalidPartOrder"
        | "InvalidRange"
        | "InvalidRequest"
        | "MissingContentLength"
        | "PreconditionFailed"
        | "UnsupportedArgument"
        | "UnsupportedSignature" => ProviderError::InvalidRequest,
        "RequestTimeTooSkewed" => ProviderError::ClockSkew,
        _ => ProviderError::Unexpected,
    }
}

#[cfg(test)]
mod tests {
    use super::provider_error_code;
    use crate::provider_capabilities::ProviderError;

    #[test]
    fn known_r2_and_s3_responses_have_safe_categories() {
        assert_eq!(
            provider_error_code("Unauthorized"),
            ProviderError::Authentication
        );
        assert_eq!(
            provider_error_code("InvalidRequest"),
            ProviderError::InvalidRequest
        );
        assert_eq!(
            provider_error_code("ServiceUnavailable"),
            ProviderError::Unavailable
        );
        assert_eq!(
            provider_error_code("RequestTimeTooSkewed"),
            ProviderError::ClockSkew
        );
    }

    #[test]
    fn unknown_service_responses_are_not_reported_as_network_failures() {
        assert_eq!(
            provider_error_code("VendorSpecificError"),
            ProviderError::Unexpected
        );
    }
}
