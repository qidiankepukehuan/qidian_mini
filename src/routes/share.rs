use crate::handler::share;
use axum::Router;
use axum::routing::{get, post};

pub fn routes() -> Router {
    Router::new()
        // 发送文件 -> POST /share
        .route("/share/get_file", post(share::share_files))
        // 发送文件列表 -> GET /share/list_file
        .route("/share/list_file", get(share::list_files))
}
