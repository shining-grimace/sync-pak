#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NetworkAccessFailure {
    PermissionMissing,
    InspectionUnavailable,
}

#[cfg(target_os = "android")]
pub(crate) fn verify() -> Result<(), NetworkAccessFailure> {
    match crate::platform::android::network_access::has_internet_permission() {
        Ok(true) => Ok(()),
        Ok(false) => Err(NetworkAccessFailure::PermissionMissing),
        Err(_) => Err(NetworkAccessFailure::InspectionUnavailable),
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn verify() -> Result<(), NetworkAccessFailure> {
    Ok(())
}
