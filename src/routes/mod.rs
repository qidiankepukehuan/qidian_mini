use crate::middleware::{cors, upload_limit, http_tracing, request_id};
use axum::Router;

mod auth;
mod health;
mod share;
mod submit;

pub fn routers() -> Router {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(submit::routes())
        .merge(share::routes())
        .layer(cors::cors_layer())
        .layer(upload_limit::body_limit_layer())
        .layer(http_tracing::trace_layer())
        .layer(request_id::request_id_layer())
}
