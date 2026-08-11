use crate::{
    providers::capabilities::{ProviderCertificateError, ProviderError, ProviderTransportError},
    providers::errors::secure_connection::general_tls_error,
};

pub(crate) fn classify(error: &rustls::Error) -> ProviderError {
    match error {
        rustls::Error::InvalidCertificate(error) => certificate_error(error),
        rustls::Error::NoCertificatesPresented | rustls::Error::UnsupportedNameType => {
            ProviderError::Certificate(ProviderCertificateError::Invalid)
        }
        rustls::Error::General(message) => transport_error(general_tls_error(message)),
        rustls::Error::AlertReceived(_) => {
            transport_error(ProviderTransportError::TlsAlertReceived)
        }
        rustls::Error::FailedToGetCurrentTime => {
            transport_error(ProviderTransportError::TlsDeviceTimeUnavailable)
        }
        rustls::Error::FailedToGetRandomBytes => {
            transport_error(ProviderTransportError::SecureRandomUnavailable)
        }
        _ => transport_error(ProviderTransportError::TlsProtocolFailed),
    }
}

fn certificate_error(error: &rustls::CertificateError) -> ProviderError {
    use rustls::CertificateError;

    let error = match error {
        CertificateError::Expired | CertificateError::ExpiredContext { .. } => {
            ProviderCertificateError::Expired
        }
        CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => {
            ProviderCertificateError::NotYetValid
        }
        CertificateError::Revoked => ProviderCertificateError::Revoked,
        CertificateError::UnknownIssuer => ProviderCertificateError::Untrusted,
        CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
            ProviderCertificateError::NameMismatch
        }
        _ => ProviderCertificateError::Invalid,
    };
    ProviderError::Certificate(error)
}

fn transport_error(error: ProviderTransportError) -> ProviderError {
    ProviderError::Transport(error)
}
