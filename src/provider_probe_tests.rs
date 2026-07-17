use super::*;

#[test]
fn non_aws_providers_require_an_endpoint() {
    let result = ProbeConfig::from_values(|name| match name {
        "SYNCPAK_PROBE_PROVIDER" => Some("cloudflare-r2".to_owned()),
        "SYNCPAK_PROBE_ACCESS_KEY_ID"
        | "SYNCPAK_PROBE_BUCKET"
        | "SYNCPAK_PROBE_PREFIX"
        | "SYNCPAK_PROBE_SECRET_ACCESS_KEY" => Some("value".to_owned()),
        _ => None,
    });

    assert!(matches!(
        result,
        Err(ProbeError::MissingSetting("SYNCPAK_PROBE_ENDPOINT"))
    ));
}

#[test]
fn prefix_is_normalized() {
    let config = ProbeConfig::from_values(|name| match name {
        "SYNCPAK_PROBE_PROVIDER" => Some("aws-s3".to_owned()),
        "SYNCPAK_PROBE_ACCESS_KEY_ID"
        | "SYNCPAK_PROBE_BUCKET"
        | "SYNCPAK_PROBE_SECRET_ACCESS_KEY" => Some("value".to_owned()),
        "SYNCPAK_PROBE_PREFIX" => Some("/isolated-test/".to_owned()),
        _ => None,
    })
    .unwrap();

    assert_eq!(config.prefix, "isolated-test");
}
