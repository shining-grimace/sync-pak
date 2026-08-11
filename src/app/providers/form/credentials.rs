use crate::{
    app::providers::save_error::ProviderPersistenceError,
    configuration::{ConfigStore, ProviderCredentials, ProviderId, ProviderRepository},
    platform::PlatformCredentialStore,
};

pub(crate) fn load(
    configuration: &ConfigStore,
    provider_id: &ProviderId,
) -> Result<ProviderCredentials, ProviderPersistenceError> {
    let store = PlatformCredentialStore::new().map_err(ProviderPersistenceError::ProtectedStore)?;
    ProviderRepository::new(configuration, &store)
        .load_credentials(provider_id)
        .map_err(ProviderPersistenceError::from)
}

pub(crate) fn resolve(
    configuration: &ConfigStore,
    provider_id: Option<&ProviderId>,
    access_key_id: &str,
    secret_access_key: &str,
    session_token: &str,
) -> Result<ProviderCredentials, ProviderPersistenceError> {
    let saved = provider_id.map(|id| load(configuration, id)).transpose()?;
    Ok(merge(
        saved.as_ref(),
        access_key_id,
        secret_access_key,
        session_token,
    ))
}

fn merge(
    saved: Option<&ProviderCredentials>,
    access_key_id: &str,
    secret_access_key: &str,
    session_token: &str,
) -> ProviderCredentials {
    ProviderCredentials {
        access_key_id: access_key_id.to_owned(),
        secret_access_key: retained(
            secret_access_key,
            saved.map(|value| &value.secret_access_key),
        ),
        session_token: optional_retained(
            session_token,
            saved.and_then(|value| value.session_token.as_ref()),
        ),
    }
}

fn retained(entered: &str, saved: Option<&String>) -> String {
    if entered.is_empty() {
        saved.cloned().unwrap_or_default()
    } else {
        entered.to_owned()
    }
}

fn optional_retained(entered: &str, saved: Option<&String>) -> Option<String> {
    if entered.is_empty() {
        saved.cloned()
    } else {
        Some(entered.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::configuration::ProviderCredentials;

    #[test]
    fn blank_secret_fields_retain_saved_values_during_editing() {
        let saved = ProviderCredentials {
            access_key_id: "old-access".into(),
            secret_access_key: "old-secret".into(),
            session_token: Some("old-token".into()),
        };

        assert!(
            merge(Some(&saved), "visible-access", "", "")
                == ProviderCredentials {
                    access_key_id: "visible-access".into(),
                    secret_access_key: "old-secret".into(),
                    session_token: Some("old-token".into()),
                }
        );
    }

    #[test]
    fn entered_secret_fields_replace_saved_values() {
        let saved = ProviderCredentials {
            access_key_id: "old-access".into(),
            secret_access_key: "old-secret".into(),
            session_token: None,
        };

        assert!(
            merge(Some(&saved), "new-access", "new-secret", "new-token")
                == ProviderCredentials {
                    access_key_id: "new-access".into(),
                    secret_access_key: "new-secret".into(),
                    session_token: Some("new-token".into()),
                }
        );
    }
}
