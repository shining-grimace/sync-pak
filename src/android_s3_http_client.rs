#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    android_certificate_verifier, android_connector_error,
    android_dns_resolver::AndroidDnsResolver,
    android_http_timeout::ConnectTimeout,
    android_http_timeout::HttpTimeout,
    android_server_certificate_verifier,
    provider_capabilities::{ProviderError, ProviderResult, ProviderTransportError},
};
use aws_smithy_runtime_api::client::{
    connector_metadata::ConnectorMetadata,
    http::{
        HttpClient, HttpConnector, HttpConnectorFuture, HttpConnectorSettings, SharedHttpClient,
        SharedHttpConnector,
    },
    orchestrator::{HttpRequest, HttpResponse},
    result::ConnectorError,
    runtime_components::RuntimeComponents,
};
use aws_smithy_types::body::SdkBody;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector as HyperHttpConnector},
    rt::{TokioExecutor, TokioTimer},
};

type AndroidTcpConnector = HyperHttpConnector<AndroidDnsResolver>;
type AndroidHttpsConnector = ConnectTimeout<HttpsConnector<AndroidTcpConnector>>;
type AndroidHyperClient = Client<AndroidHttpsConnector, SdkBody>;

pub fn build() -> ProviderResult<SharedHttpClient> {
    if !android_certificate_verifier::is_initialized() {
        return Err(ProviderError::Transport(
            ProviderTransportError::TrustStoreUnavailable,
        ));
    }
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let verifier = android_server_certificate_verifier::build(provider.clone())
        .map_err(|_| ProviderError::Transport(ProviderTransportError::TrustStoreUnavailable))?;
    let tls_config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|_| ProviderError::Transport(ProviderTransportError::TlsConfigurationFailed))?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok(SharedHttpClient::new(AndroidHttpClient {
        tls_config,
        connectors: Mutex::new(HashMap::new()),
    }))
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
struct ConnectorKey {
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
}

impl From<&HttpConnectorSettings> for ConnectorKey {
    fn from(settings: &HttpConnectorSettings) -> Self {
        Self {
            connect_timeout: settings.connect_timeout(),
            read_timeout: settings.read_timeout(),
        }
    }
}

struct AndroidHttpClient {
    tls_config: rustls::ClientConfig,
    connectors: Mutex<HashMap<ConnectorKey, SharedHttpConnector>>,
}

impl std::fmt::Debug for AndroidHttpClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AndroidHttpClient").finish()
    }
}

impl HttpClient for AndroidHttpClient {
    fn http_connector(
        &self,
        settings: &HttpConnectorSettings,
        _components: &RuntimeComponents,
    ) -> SharedHttpConnector {
        let key = ConnectorKey::from(settings);
        let mut connectors = match self.connectors.lock() {
            Ok(connectors) => connectors,
            Err(poisoned) => poisoned.into_inner(),
        };
        connectors
            .entry(key)
            .or_insert_with(|| {
                SharedHttpConnector::new(AndroidHttpConnector::new(self.tls_config.clone(), key))
            })
            .clone()
    }

    fn connector_metadata(&self) -> Option<ConnectorMetadata> {
        Some(ConnectorMetadata::new("hyper", Some(Cow::Borrowed("1.x"))))
    }
}

#[derive(Clone)]
struct AndroidHttpConnector {
    client: AndroidHyperClient,
    read_timeout: Option<Duration>,
}

impl AndroidHttpConnector {
    fn new(tls_config: rustls::ClientConfig, settings: ConnectorKey) -> Self {
        let mut tcp = HyperHttpConnector::new_with_resolver(AndroidDnsResolver::new());
        tcp.enforce_http(false);
        tcp.set_nodelay(true);
        let https = HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_only()
            .enable_all_versions()
            .wrap_connector(tcp);
        let mut builder = Client::builder(TokioExecutor::new());
        builder.pool_timer(TokioTimer::new());
        Self {
            client: builder.build(ConnectTimeout::new(https, settings.connect_timeout)),
            read_timeout: settings.read_timeout,
        }
    }
}

impl std::fmt::Debug for AndroidHttpConnector {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AndroidHttpConnector")
            .field("read_timeout", &self.read_timeout)
            .finish()
    }
}

impl HttpConnector for AndroidHttpConnector {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let request = match request.try_into_http1x() {
            Ok(request) => request,
            Err(error) => {
                return HttpConnectorFuture::ready(Err(ConnectorError::user(error.into())));
            }
        };
        if request.uri().scheme_str() != Some("https") {
            let error = std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "provider endpoint must use HTTPS",
            );
            return HttpConnectorFuture::ready(Err(ConnectorError::user(Box::new(error))));
        }
        let client = self.client.clone();
        let read_timeout = self.read_timeout;
        HttpConnectorFuture::new(async move {
            let response = match read_timeout {
                Some(duration) => tokio::time::timeout(duration, client.request(request))
                    .await
                    .map_err(|_| ConnectorError::timeout(Box::new(HttpTimeout::read(duration))))?,
                None => client.request(request).await,
            }
            .map_err(android_connector_error::classify)?
            .map(SdkBody::from_body_1_x);
            HttpResponse::try_from(response)
                .map_err(|error| ConnectorError::other(error.into(), None))
        })
    }
}
