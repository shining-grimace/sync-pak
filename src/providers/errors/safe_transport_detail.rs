#[cfg(any(target_os = "android", test))]
use std::error::Error;

#[cfg(any(target_os = "android", test))]
const MAX_CAUSES: usize = 8;
#[cfg(any(target_os = "android", test))]
const MAX_DETAIL_LENGTH: usize = 320;

/// A bounded transport-error description with endpoint- and credential-shaped values removed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeTransportDetail(Box<str>);

impl SafeTransportDetail {
    #[cfg(any(target_os = "android", test))]
    pub(crate) fn from_error_chain(error: &(dyn Error + 'static)) -> Self {
        let mut causes = Vec::new();
        let mut current = Some(error);
        while let Some(error) = current.filter(|_| causes.len() < MAX_CAUSES) {
            let cause = sanitize(&error.to_string());
            if !cause.is_empty() && causes.last() != Some(&cause) {
                causes.push(cause);
            }
            current = error.source();
        }
        let detail = truncate(&causes.join("; caused by: "));
        if detail.is_empty() {
            Self("socket error supplied no description".into())
        } else {
            Self(detail.into())
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(any(target_os = "android", test))]
fn sanitize(message: &str) -> String {
    message
        .split_whitespace()
        .map(sanitize_token)
        .fold(String::new(), |mut output, token| {
            if !output.is_empty() {
                output.push(' ');
            }
            output.push_str(&token);
            output
        })
}

#[cfg(any(target_os = "android", test))]
fn sanitize_token(token: &str) -> String {
    let token: String = token
        .chars()
        .filter(|character| character.is_ascii_graphic())
        .collect();
    let value = token.trim_matches(|character: char| {
        matches!(
            character,
            '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':' | '.' | '"' | '\''
        )
    });
    if is_private_value(value) {
        "[redacted]".to_owned()
    } else {
        token
    }
}

#[cfg(any(target_os = "android", test))]
fn is_private_value(value: &str) -> bool {
    value.contains("://")
        || value.contains('@')
        || value.contains('/')
        || value.contains('\\')
        || looks_like_dotted_address(value)
        || value.len() > 48
        || looks_like_socket_address(value)
        || looks_like_identifier(value)
}

#[cfg(any(target_os = "android", test))]
fn looks_like_dotted_address(value: &str) -> bool {
    value.split('.').count() >= 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b':' | b'_'))
}

#[cfg(any(target_os = "android", test))]
fn looks_like_socket_address(value: &str) -> bool {
    value.contains(':') && value.bytes().any(|byte| byte.is_ascii_digit())
}

#[cfg(any(target_os = "android", test))]
fn looks_like_identifier(value: &str) -> bool {
    value.len() >= 16
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
        && value.bytes().any(|byte| byte.is_ascii_alphabetic())
        && value.bytes().any(|byte| byte.is_ascii_digit())
}

#[cfg(any(target_os = "android", test))]
fn truncate(value: &str) -> String {
    let mut output: String = value.chars().take(MAX_DETAIL_LENGTH).collect();
    if value.chars().count() > MAX_DETAIL_LENGTH {
        output.push_str("...");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{MAX_DETAIL_LENGTH, SafeTransportDetail};

    #[test]
    fn preserves_socket_failure_context() {
        let source = std::io::Error::other("handshake read failed: protocol error 71");
        let detail = SafeTransportDetail::from_error_chain(&source);

        assert!(detail.as_str().contains("handshake read failed"));
        assert!(detail.as_str().contains("error 71"));
    }

    #[test]
    fn removes_endpoint_and_credential_shaped_values() {
        let source = std::io::Error::other(
            "request to https://account.example.com failed for ABCD1234EFGH5678IJKL at 192.0.2.1:443",
        );
        let detail = SafeTransportDetail::from_error_chain(&source);

        assert!(!detail.as_str().contains("account.example.com"));
        assert!(!detail.as_str().contains("ABCD1234EFGH5678IJKL"));
        assert!(!detail.as_str().contains("192.0.2.1"));
        assert!(detail.as_str().contains("[redacted]"));
    }

    #[test]
    fn bounds_dependency_error_text() {
        let source = std::io::Error::other("failure ".repeat(100));
        let detail = SafeTransportDetail::from_error_chain(&source);

        assert!(detail.as_str().chars().count() <= MAX_DETAIL_LENGTH + 3);
        assert!(detail.as_str().ends_with("..."));
    }
}
