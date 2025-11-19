use crate::middleware::request_id::RequestId;
use axum::body::Body;
use axum::http::Request;
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use tracing::{Level, Span};
use uuid::Uuid;

pub type HttpMakeSpanFn = fn(&Request<Body>) -> Span;

pub type HttpTraceLayer = TraceLayer<HttpMakeClassifier, HttpMakeSpanFn>;

pub fn trace_layer() -> HttpTraceLayer {
    fn make_span(req: &Request<Body>) -> Span {
        let rid = req
            .extensions()
            .get::<RequestId>()
            .copied()
            .unwrap_or_else(|| RequestId(Uuid::new_v4()));

        tracing::span!(
            Level::DEBUG,
            "request",
            request_id = display(rid),
            method = display(req.method()),
            uri = display(req.uri()),
            version = debug(req.version()),
        )
    }

    TraceLayer::new_for_http().make_span_with(make_span as HttpMakeSpanFn)
}
