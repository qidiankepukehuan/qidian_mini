use crate::middleware::{cors, http_tracing, request_id, upload_limit};
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
