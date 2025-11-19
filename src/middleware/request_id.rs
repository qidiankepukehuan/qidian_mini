use axum::{body::Body, http::Request};
use std::convert::Infallible;
use std::fmt;
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;
use uuid::Uuid;

#[derive(Clone, Debug, Copy)]
pub struct RequestId(pub Uuid);

impl RequestId {
    pub fn new() -> Self {
        RequestId(Uuid::new_v4())
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Uuid> for RequestId {
    fn from(id: Uuid) -> Self {
        RequestId(id)
    }
}

#[derive(Clone, Debug)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdService { inner }
    }
}

#[derive(Clone, Debug)]
pub struct RequestIdService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for RequestIdService<S>
where
    S: Service<Request<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Infallible>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        // 这里生成一个 RequestId 并塞进 extensions
        let rid = RequestId::new();
        req.extensions_mut().insert(rid);

        self.inner.call(req)
    }
}

pub fn request_id_layer() -> RequestIdLayer {
    RequestIdLayer
}
