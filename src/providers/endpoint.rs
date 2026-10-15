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

/// Only infer signing regions from Backblaze's documented service hostnames.
pub(crate) fn b2_region(endpoint: &str) -> Option<String> {
    let endpoint = normalize(endpoint)?;
    let host = endpoint.strip_prefix("https://")?.trim_end_matches('/');
    let region = host.strip_prefix("s3.")?.strip_suffix(".backblazeb2.com")?;
    (!region.is_empty()
        && region
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'))
    .then(|| region.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{b2_region, normalize};

    #[test]
    fn only_standard_b2_service_endpoints_supply_a_region() {
        assert_eq!(
            b2_region("s3.us-west-004.backblazeb2.com/"),
            Some("us-west-004".into())
        );
        for endpoint in [
            "https://s3.us-west-004.backblazeb2.com.evil.example",
            "https://bucket.s3.us-west-004.backblazeb2.com",
            "https://s3.us-west-004.backblazeb2.com/prefix",
            "https://proxy.example",
            "https://s3..backblazeb2.com",
        ] {
            assert_eq!(b2_region(endpoint), None);
        }
    }

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
