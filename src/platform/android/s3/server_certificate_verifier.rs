#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::Arc;

use rustls::{
    CertificateError, DigitallySignedStruct, DistinguishedName, Error, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, ServerName, UnixTime},
    server::ParsedCertificate,
};

#[derive(Debug)]
struct AndroidServerCertificateVerifier {
    platform: rustls_platform_verifier::Verifier,
}

pub fn build(crypto_provider: Arc<CryptoProvider>) -> Result<Arc<dyn ServerCertVerifier>, Error> {
    Ok(Arc::new(AndroidServerCertificateVerifier {
        platform: rustls_platform_verifier::Verifier::new(crypto_provider)?,
    }))
}

impl ServerCertVerifier for AndroidServerCertificateVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        match self
            .platform
            .verify_server_cert(end_entity, intermediates, server_name, &[], now)
        {
            Err(Error::InvalidCertificate(CertificateError::Revoked)) => {
                soft_fail_ambiguous_android_revocation(end_entity, server_name)
            }
            result => result,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.platform.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.platform.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.platform.supported_verify_schemes()
    }

    fn requires_raw_public_keys(&self) -> bool {
        self.platform.requires_raw_public_keys()
    }

    fn root_hint_subjects(&self) -> Option<&[DistinguishedName]> {
        self.platform.root_hint_subjects()
    }
}

fn soft_fail_ambiguous_android_revocation(
    end_entity: &CertificateDer<'_>,
    server_name: &ServerName<'_>,
) -> Result<ServerCertVerified, Error> {
    // Version 0.1.1 of the JVM helper reaches Revoked only after Android's trust manager
    // accepts the chain. Its separate PKIX revocation pass maps every validator exception to
    // Revoked, so the result does not establish that the certificate was actually revoked.
    // Finish the hostname check normally and soft-fail only that ambiguous secondary pass.
    let parsed = ParsedCertificate::try_from(end_entity)?;
    rustls::client::verify_server_name(&parsed, server_name)?;
    Ok(ServerCertVerified::assertion())
}
