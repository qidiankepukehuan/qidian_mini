use axum::http::{HeaderValue, Method};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

pub fn cors_layer() -> CorsLayer {
    // 检查启动参数是否包含 "--test"
    let is_test = std::env::args().any(|arg| arg == "--test");

    let mut cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(vec![
            "https://qidian.space".parse::<HeaderValue>().unwrap(),
            "https://mini.qidian.space".parse::<HeaderValue>().unwrap(),
            "https://contribute.qidian.space"
                .parse::<HeaderValue>()
                .unwrap(),
        ]))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    if is_test {
        cors = CorsLayer::new()
            .allow_origin(AllowOrigin::any()) // 允许所有来源
            .allow_methods(Any) // 允许所有 HTTP 方法
            .allow_headers(Any) // 允许所有头
            .allow_credentials(true) // 允许携带凭证
    }

    cors
}
