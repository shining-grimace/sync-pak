#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::{
    configuration::{ProviderConfig, ProviderCredentials},
    providers::verification::ProviderVerification,
    providers::verification::failure::VerificationFailure,
    providers::verification::panic::VerificationPanic,
};

pub(crate) fn verify(
    provider: &ProviderConfig,
    credentials: ProviderCredentials,
) -> Result<ProviderVerification, VerificationFailure> {
    let private_values = private_values(provider, &credentials);
    match catch_unwind(AssertUnwindSafe(|| verify_inner(provider, credentials))) {
        Ok(result) => result,
        Err(payload) => Err(VerificationFailure::WorkerPanicked(
            VerificationPanic::inspect(payload.as_ref(), &private_values),
        )),
    }
}

fn verify_inner(
    provider: &ProviderConfig,
    credentials: ProviderCredentials,
) -> Result<ProviderVerification, VerificationFailure> {
    crate::providers::verification::network_access::verify().map_err(VerificationFailure::from)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| VerificationFailure::RuntimeInitialization)?;
    runtime
        .block_on(crate::providers::s3::verification::verify_s3_provider(
            provider,
            credentials,
        ))
        .map_err(VerificationFailure::from)
}

fn private_values(provider: &ProviderConfig, credentials: &ProviderCredentials) -> Vec<String> {
    let mut values = vec![
        provider.id.as_str().to_owned(),
        provider.name.clone(),
        credentials.access_key_id.clone(),
        credentials.secret_access_key.clone(),
    ];
    values.extend(credentials.session_token.iter().cloned());
    values.extend(provider.options.account_id.iter().cloned());
    values.extend(provider.options.default_bucket.iter().cloned());
    values.extend(provider.options.endpoint.iter().cloned());
    values.extend(provider.options.region.iter().cloned());
    values
}
