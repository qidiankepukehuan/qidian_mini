use axum::Router;
use axum::routing::post;
use crate::handler::auth;

pub fn routes() -> Router {
    Router::new()
        // 发送验证码 -> POST /auth/code/send
        .route("/auth/send", post(auth::send_code))
        // 验证验证码 -> POST /auth/code/verify
        .route("/auth/verify", post(auth::verify_code))
}