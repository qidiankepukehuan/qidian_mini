use crate::handler::submit;
use axum::Router;
use axum::routing::post;

pub fn routes() -> Router {
    Router::new().route("/submit", post(submit::submit_article))
}