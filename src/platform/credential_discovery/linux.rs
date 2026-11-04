use super::{CapabilityError, CredentialBackend, CredentialProvider};
pub(crate) fn discover() -> Result<Vec<CredentialProvider>, CapabilityError> {
    use crate::platform::{
        credential_deadline::{SERVICE_TIMEOUT, within},
        credential_errors::map_bus_error,
    };
    use zbus::zvariant::OwnedObjectPath;
    async_io::block_on(within(std::time::Duration::from_secs(20), async {
        let connection = within(SERVICE_TIMEOUT, async {
            zbus::Connection::session().await.map_err(map_bus_error)
        })
        .await?;
        let result = async {
            let bus = zbus::Proxy::new(
                &connection,
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
            )
            .await
            .map_err(map_bus_error)?;
            let names: Vec<String> = bus.call("ListNames", &()).await.map_err(map_bus_error)?;
            let activatable: Vec<String> = bus
                .call("ListActivatableNames", &())
                .await
                .map_err(map_bus_error)?;
            let mut owners = std::collections::HashSet::new();
            let mut providers = Vec::new();
            for backend in [
                CredentialBackend::Kde,
                CredentialBackend::KdeLegacy6,
                CredentialBackend::KdeLegacy5,
                CredentialBackend::Gnome,
                CredentialBackend::System,
            ] {
                let destination = backend.destination();
                if !names.iter().any(|n| n == destination)
                    && !activatable.iter().any(|n| n == destination)
                {
                    continue;
                }
                let probe = within(SERVICE_TIMEOUT, async {
                    let service = zbus::Proxy::new(
                        &connection,
                        destination,
                        "/org/freedesktop/secrets",
                        "org.freedesktop.Secret.Service",
                    )
                    .await
                    .map_err(map_bus_error)?;
                    let path: OwnedObjectPath = service
                        .call("ReadAlias", &("default",))
                        .await
                        .map_err(map_bus_error)?;
                    let configured = path.as_str() != "/";
                    let locked = if configured {
                        zbus::Proxy::new(
                            &connection,
                            destination,
                            path,
                            "org.freedesktop.Secret.Collection",
                        )
                        .await
                        .map_err(map_bus_error)?
                        .get_property::<bool>("Locked")
                        .await
                        .map_err(map_bus_error)?
                    } else {
                        false
                    };
                    let owner: String = bus
                        .call("GetNameOwner", &(destination,))
                        .await
                        .map_err(map_bus_error)?;
                    let mut identity = backend;
                    if backend == CredentialBackend::System {
                        for candidate in [CredentialBackend::Kde, CredentialBackend::Gnome] {
                            let known: Result<String, _> =
                                bus.call("GetNameOwner", &(candidate.destination(),)).await;
                            if known.as_ref().is_ok_and(|known| known == &owner) {
                                identity = candidate;
                                break;
                            }
                        }
                    }
                    Ok((
                        owner,
                        CredentialProvider {
                            backend: identity,
                            configured,
                            locked,
                        },
                    ))
                })
                .await;
                if let Ok((owner, provider)) = probe
                    && owners.insert(owner)
                {
                    providers.push(provider);
                }
            }
            Ok(providers)
        }
        .await;
        let _ = within(std::time::Duration::from_secs(1), async {
            connection.close().await.map_err(map_bus_error)
        })
        .await;
        result
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Metadata only: never unlocks a collection or reads/writes credential items.
    #[test]
    #[ignore = "requires a real desktop session bus"]
    fn desktop_endpoints_keep_encrypted_sessions_and_collection_routing_separate() {
        use crate::platform::credential_deadline::{SERVICE_TIMEOUT, within};
        use crate::platform::credential_errors::{map_bus_error, map_error};
        let providers = discover().unwrap();
        assert!(!providers.is_empty());
        for provider in providers.iter().filter(|p| p.configured) {
            println!("{}", provider.description());
            async_io::block_on(within(SERVICE_TIMEOUT, async {
                let connection = zbus::Connection::session().await.map_err(map_bus_error)?;
                let result = async {
                    let raw = zbus::Proxy::new(
                        &connection,
                        provider.backend.destination(),
                        "/org/freedesktop/secrets",
                        "org.freedesktop.Secret.Service",
                    )
                    .await
                    .map_err(map_bus_error)?;
                    let expected: zbus::zvariant::OwnedObjectPath = raw
                        .call("ReadAlias", &("default",))
                        .await
                        .map_err(map_bus_error)?;
                    let service = secret_service::SecretService::connect_with_destination(
                        secret_service::EncryptionType::Dh,
                        connection.clone(),
                        provider.backend.destination(),
                    )
                    .await
                    .map_err(map_error)?;
                    let collection = service.get_default_collection().await.map_err(map_error)?;
                    assert_eq!(collection.collection_path, expected);
                    Ok(())
                }
                .await;
                let _ = connection.close().await;
                result
            }))
            .unwrap();
        }
        let selected = super::super::preferred(
            &providers,
            &std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        );
        println!("Automatic selection: {selected:?}");
    }
}
