use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::{
    configuration::{ProviderConfig, ProviderCredentials},
    provider_verification::ProviderVerification,
    provider_verification_failure::VerificationFailure,
};

pub(crate) fn verify(
    provider: &ProviderConfig,
    credentials: ProviderCredentials,
) -> Result<ProviderVerification, VerificationFailure> {
    match catch_unwind(AssertUnwindSafe(|| verify_inner(provider, credentials))) {
        Ok(result) => result,
        Err(payload) => Err(VerificationFailure::from_panic(payload.as_ref())),
    }
}

fn verify_inner(
    provider: &ProviderConfig,
    credentials: ProviderCredentials,
) -> Result<ProviderVerification, VerificationFailure> {
    crate::provider_network_access::verify().map_err(VerificationFailure::from)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| VerificationFailure::RuntimeInitialization)?;
    runtime
        .block_on(crate::s3_provider_verification::verify_s3_provider(
            provider,
            credentials,
        ))
        .map_err(VerificationFailure::from)
}
