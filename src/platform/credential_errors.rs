use crate::capabilities::CapabilityError;

pub(super) fn map_error(error: secret_service::Error) -> CapabilityError {
    match error {
        secret_service::Error::NoResult => CapabilityError::NotFound,
        secret_service::Error::Locked => CapabilityError::Locked,
        secret_service::Error::Prompt => CapabilityError::Cancelled,
        secret_service::Error::PromptDisconnected | secret_service::Error::Unavailable => {
            CapabilityError::Unavailable
        }
        secret_service::Error::Zbus(error) => map_bus_error(error),
        secret_service::Error::ZbusFdo(error) => map_bus_error(error.into()),
        _ => CapabilityError::Unexpected,
    }
}

pub(super) fn map_bus_error(error: zbus::Error) -> CapabilityError {
    match error {
        zbus::Error::MethodError(name, _, _) => match name.as_str() {
            "org.freedesktop.DBus.Error.ServiceUnknown"
            | "org.freedesktop.DBus.Error.NameHasNoOwner" => CapabilityError::ServiceMissing,
            "org.freedesktop.DBus.Error.AccessDenied" | "org.freedesktop.DBus.Error.AuthFailed" => {
                CapabilityError::AccessDenied
            }
            "org.freedesktop.DBus.Error.NoReply"
            | "org.freedesktop.DBus.Error.TimedOut"
            | "org.freedesktop.DBus.Error.Timeout" => CapabilityError::TimedOut,
            "org.freedesktop.Secret.Error.IsLocked" => CapabilityError::Locked,
            _ => CapabilityError::Unavailable,
        },
        zbus::Error::FDO(error) => match *error {
            zbus::fdo::Error::ServiceUnknown(_) | zbus::fdo::Error::NameHasNoOwner(_) => {
                CapabilityError::ServiceMissing
            }
            zbus::fdo::Error::AccessDenied(_) | zbus::fdo::Error::AuthFailed(_) => {
                CapabilityError::AccessDenied
            }
            zbus::fdo::Error::NoReply(_)
            | zbus::fdo::Error::TimedOut(_)
            | zbus::fdo::Error::Timeout(_) => CapabilityError::TimedOut,
            _ => CapabilityError::Unavailable,
        },
        _ => CapabilityError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_denied_locked_and_cancelled_storage_separately() {
        assert_eq!(
            map_error(secret_service::Error::ZbusFdo(
                zbus::fdo::Error::ServiceUnknown("".into())
            )),
            CapabilityError::ServiceMissing
        );
        assert_eq!(
            map_error(secret_service::Error::ZbusFdo(
                zbus::fdo::Error::AccessDenied("".into())
            )),
            CapabilityError::AccessDenied
        );
        assert_eq!(
            map_error(secret_service::Error::Locked),
            CapabilityError::Locked
        );
        assert_eq!(
            map_error(secret_service::Error::Prompt),
            CapabilityError::Cancelled
        );
    }
}
