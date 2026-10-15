/// Defaults custom endpoints to HTTPS while preserving explicit protocols.
pub(crate) fn normalize(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else if value.contains("://") {
        Some(value.to_owned())
    } else {
        Some(format!(
            "https://{}",
            value.strip_prefix("//").unwrap_or(value)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn defaults_bare_hosts_ports_and_paths_to_https() {
        for endpoint in [
            "s3.us-west-004.backblazeb2.com",
            "localhost:9000",
            "[::1]:9000/prefix",
            "storage.example/prefix",
        ] {
            assert_eq!(normalize(endpoint), Some(format!("https://{endpoint}")));
        }
        assert_eq!(
            normalize(" //storage.example "),
            Some("https://storage.example".into())
        );
    }

    #[test]
    fn preserves_explicit_protocols_and_ignores_blank_values() {
        for endpoint in [
            "https://storage.example",
            "http://localhost:9000",
            "ftp://storage.example",
        ] {
            assert_eq!(normalize(endpoint), Some(endpoint.into()));
            assert_eq!(normalize(&format!(" {endpoint} ")), Some(endpoint.into()));
        }
        assert_eq!(normalize("  "), None);
    }
}
