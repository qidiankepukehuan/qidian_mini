use axum::Router;
use crate::middleware::cors;
use crate::middleware::upload_limit;

mod health;
mod submit;
mod auth;

pub fn routers() -> Router {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(submit::routes())
        .layer(cors::cors_layer())
        .layer(upload_limit::body_limit_layer())
}