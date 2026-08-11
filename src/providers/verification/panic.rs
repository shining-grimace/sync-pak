#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::any::Any;

const MAX_DETAIL_CHARACTERS: usize = 800;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PanicCategory {
    Runtime,
    SecureConnection,
    Unexpected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerificationPanic {
    category: PanicCategory,
    technical_details: String,
}

impl VerificationPanic {
    pub(crate) fn inspect(payload: &(dyn Any + Send), private_values: &[String]) -> Self {
        let raw_message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
        let category = raw_message.map_or(PanicCategory::Unexpected, classify);
        let message = raw_message.map_or_else(
            || "non-text panic payload".to_owned(),
            |message| sanitize(message, private_values),
        );
        let context = match category {
            PanicCategory::Runtime => "provider runtime dependency panicked",
            PanicCategory::SecureConnection => "secure-connection dependency panicked",
            PanicCategory::Unexpected => "provider verification dependency panicked",
        };
        Self {
            category,
            technical_details: format!("{context}: {message}"),
        }
    }

    pub(crate) fn technical_details(&self) -> &str {
        &self.technical_details
    }

    pub(crate) fn message(&self) -> &'static str {
        match self.category {
            PanicCategory::Runtime => {
                "A provider runtime dependency failed unexpectedly. Open Diagnostics for its redacted failure details."
            }
            PanicCategory::SecureConnection => {
                "A secure-connection dependency failed unexpectedly. Open Diagnostics for its redacted failure details."
            }
            PanicCategory::Unexpected => {
                "A provider dependency failed unexpectedly. Open Diagnostics for its redacted failure details."
            }
        }
    }
}

fn classify(message: &str) -> PanicCategory {
    let message = message.to_ascii_lowercase();
    if message.contains("cryptoprovider")
        || message.contains("crypto provider")
        || message.contains("rustls")
        || message.contains("tls")
    {
        PanicCategory::SecureConnection
    } else if message.contains("tokio")
        || message.contains("runtime")
        || message.contains("reactor")
    {
        PanicCategory::Runtime
    } else {
        PanicCategory::Unexpected
    }
}

fn sanitize(message: &str, private_values: &[String]) -> String {
    let mut message = message.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut private_values = private_values
        .iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    private_values.sort_unstable_by_key(|value| std::cmp::Reverse(value.len()));
    for value in private_values {
        message = message.replace(value.as_str(), "[REDACTED]");
    }
    let mut characters = message.chars();
    let shortened = characters
        .by_ref()
        .take(MAX_DETAIL_CHARACTERS)
        .collect::<String>();
    if characters.next().is_some() {
        format!("{shortened}…")
    } else if shortened.is_empty() {
        "empty panic message".to_owned()
    } else {
        shortened
    }
}

#[cfg(test)]
mod tests {
    use super::VerificationPanic;

    #[test]
    fn preserves_failure_mode_while_redacting_private_values() {
        let payload = String::from(
            "request for https://account.example/bucket failed with access-key and secret-value",
        );
        let private_values = vec![
            "https://account.example".to_owned(),
            "bucket".to_owned(),
            "access-key".to_owned(),
            "secret-value".to_owned(),
        ];

        let failure = VerificationPanic::inspect(&payload, &private_values);

        assert!(failure.technical_details().contains("request for"));
        assert!(failure.technical_details().contains("[REDACTED]"));
        assert!(!failure.technical_details().contains("account.example"));
        assert!(!failure.technical_details().contains("access-key"));
        assert!(!failure.technical_details().contains("secret-value"));
    }

    #[test]
    fn identifies_runtime_and_secure_connection_panics() {
        let runtime = String::from("Tokio reactor is unavailable");
        let secure = "no process-level CryptoProvider available";

        assert!(
            VerificationPanic::inspect(&runtime, &[])
                .technical_details()
                .starts_with("provider runtime dependency panicked")
        );
        assert!(
            VerificationPanic::inspect(&secure, &[])
                .technical_details()
                .starts_with("secure-connection dependency panicked")
        );
    }
}
