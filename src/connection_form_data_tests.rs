use crate::configuration::{
    CredentialReference, ProviderConfig, ProviderId, ProviderKind, ProviderOptions,
};

use super::provider_bucket;

fn provider(default_bucket: Option<&str>) -> ProviderConfig {
    let id = ProviderId::new();
    ProviderConfig {
        id: id.clone(),
        credential_reference: CredentialReference { provider_id: id },
        name: "Cloud".into(),
        kind: ProviderKind::AwsS3,
        options: ProviderOptions {
            account_id: None,
            default_bucket: default_bucket.map(str::to_owned),
            endpoint: None,
            region: Some("ap-southeast-2".into()),
        },
    }
}

#[test]
fn provider_default_bucket_is_available_only_for_a_valid_selection() {
    let providers = [provider(Some("new-default")), provider(None)];

    assert_eq!(provider_bucket(&providers, 0), Some("new-default"));
    assert_eq!(provider_bucket(&providers, 1), None);
    assert_eq!(provider_bucket(&providers, -1), None);
}
