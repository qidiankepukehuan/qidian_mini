use axum::Router;
use crate::middleware::cors;

mod health;
mod submit;
mod auth;

pub fn routers() -> Router {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(submit::routes())
        .layer(cors::cors_layer())
}