use super::Discovery;
use crate::platform::credential_deadline::{SERVICE_TIMEOUT, within};
use crate::platform::credential_errors::map_bus_error;
use zbus::{Connection, Proxy, zvariant::OwnedObjectPath};

pub(super) async fn inspect(
    connection: &Connection,
    bus: &Proxy<'_>,
    names: &[String],
) -> Discovery {
    let system_service = within(SERVICE_TIMEOUT, async {
        let pid: u32 = bus
            .call("GetConnectionUnixProcessID", &("org.freedesktop.secrets",))
            .await
            .map_err(map_bus_error)?;
        let executable = std::fs::read_link(format!("/proc/{pid}/exe"))
            .map_err(|_| crate::capabilities::CapabilityError::Unavailable)?;
        Ok(
            match executable.file_name().and_then(|name| name.to_str()) {
                Some("oo7-daemon") => Some("oo7".into()),
                Some("ksecretd" | "kwalletd5" | "kwalletd6") => Some("KDE Wallet".into()),
                Some("gnome-keyring-daemon") => Some("GNOME Keyring".into()),
                _ => None,
            },
        )
    })
    .await
    .ok()
    .flatten();

    let oo7_inactive = system_service.as_deref() != Some("oo7")
        && names.iter().any(|name| name == "org.freedesktop.systemd1")
        && [
            "/usr/libexec/oo7-daemon",
            "/usr/lib/oo7-daemon",
            "/usr/bin/oo7-daemon",
        ]
        .iter()
        .any(|path| std::path::Path::new(path).is_file())
        && within(SERVICE_TIMEOUT, async {
            let owner: String = bus
                .call("GetNameOwner", &("org.freedesktop.systemd1",))
                .await
                .map_err(map_bus_error)?;
            let manager = Proxy::new(
                connection,
                owner.as_str(),
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
            )
            .await
            .map_err(map_bus_error)?;
            let path: OwnedObjectPath =
                match manager.call("GetUnit", &("oo7-daemon.service",)).await {
                    Ok(path) => path,
                    Err(zbus::Error::MethodError(name, _, _))
                        if name.as_str() == "org.freedesktop.systemd1.NoSuchUnit" =>
                    {
                        return Ok(true);
                    }
                    Err(error) => return Err(map_bus_error(error)),
                };
            let unit = Proxy::new(connection, owner, path, "org.freedesktop.systemd1.Unit")
                .await
                .map_err(map_bus_error)?;
            let state: String = unit
                .get_property("ActiveState")
                .await
                .map_err(map_bus_error)?;
            Ok(matches!(state.as_str(), "inactive" | "failed"))
        })
        .await
        .unwrap_or(false);

    let mut inactive_service_addresses = Vec::new();
    if let Some(owner) = &system_service {
        for (label, executable, addresses) in [
            (
                "GNOME Keyring",
                "/usr/bin/gnome-keyring-daemon",
                &["org.gnome.keyring"][..],
            ),
            (
                "KDE Wallet",
                "/usr/bin/ksecretd",
                &[
                    "org.kde.secretservicecompat",
                    "org.kde.kwalletd6",
                    "org.kde.kwalletd5",
                ][..],
            ),
        ] {
            if owner != label
                && std::path::Path::new(executable).is_file()
                && !names.iter().any(|name| addresses.contains(&name.as_str()))
            {
                inactive_service_addresses.push(label);
            }
        }
    }
    Discovery {
        system_service,
        oo7_inactive,
        inactive_service_addresses,
        ..Discovery::default()
    }
}
