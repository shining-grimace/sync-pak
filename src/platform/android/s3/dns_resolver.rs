#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use hyper_util::client::legacy::connect::dns::{GaiAddrs, GaiResolver, Name};
use tower_service::Service;

use crate::providers::errors::endpoint_resolution::EndpointResolutionError;

#[derive(Clone, Debug)]
pub(crate) struct AndroidDnsResolver {
    inner: GaiResolver,
}

impl AndroidDnsResolver {
    pub(crate) fn new() -> Self {
        Self {
            inner: GaiResolver::new(),
        }
    }
}

impl Service<Name> for AndroidDnsResolver {
    type Response = GaiAddrs;
    type Error = EndpointResolutionError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(context)
            .map_err(EndpointResolutionError::new)
    }

    fn call(&mut self, name: Name) -> Self::Future {
        let future = self.inner.call(name);
        Box::pin(async move { future.await.map_err(EndpointResolutionError::new) })
    }
}
