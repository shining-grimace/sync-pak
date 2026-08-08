#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{
    error::Error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use aws_smithy_runtime_api::box_error::BoxError;
use tower_service::Service;

#[derive(Clone, Debug)]
pub(crate) struct ConnectTimeout<C> {
    inner: C,
    duration: Option<Duration>,
}

impl<C> ConnectTimeout<C> {
    pub(crate) fn new(inner: C, duration: Option<Duration>) -> Self {
        Self { inner, duration }
    }
}

impl<C> Service<hyper::Uri> for ConnectTimeout<C>
where
    C: Service<hyper::Uri> + Send + 'static,
    C::Future: Send + 'static,
    C::Response: Send + 'static,
    C::Error: Into<BoxError>,
{
    type Response = C::Response;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context).map_err(Into::into)
    }

    fn call(&mut self, uri: hyper::Uri) -> Self::Future {
        let future = self.inner.call(uri);
        let duration = self.duration;
        Box::pin(async move {
            match duration {
                Some(duration) => tokio::time::timeout(duration, future)
                    .await
                    .map_err(|_| Box::new(HttpTimeout::connect(duration)) as BoxError)?
                    .map_err(Into::into),
                None => future.await.map_err(Into::into),
            }
        })
    }
}

#[derive(Debug)]
pub(crate) struct HttpTimeout {
    phase: &'static str,
    duration: Duration,
}

impl HttpTimeout {
    pub(crate) fn connect(duration: Duration) -> Self {
        Self {
            phase: "connect",
            duration,
        }
    }

    pub(crate) fn read(duration: Duration) -> Self {
        Self {
            phase: "read",
            duration,
        }
    }
}

impl std::fmt::Display for HttpTimeout {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "HTTP {} timeout after {:?}",
            self.phase, self.duration
        )
    }
}

impl Error for HttpTimeout {}
