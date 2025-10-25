use crate::middleware::cors;
use crate::middleware::upload_limit;
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
}
