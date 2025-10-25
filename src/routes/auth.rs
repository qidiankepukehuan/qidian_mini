use crate::handler::auth;
use axum::Router;
use axum::routing::post;

pub fn routes() -> Router {
    Router::new()
        // 发送验证码 -> POST /auth/send
        .route("/auth/send", post(auth::send_code))
}
