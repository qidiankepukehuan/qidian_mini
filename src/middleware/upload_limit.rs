use axum::extract::DefaultBodyLimit;

/// 上传大小的单位换算常量：1 MB
pub const MB: usize = 1024 * 1024;
/// 请求体大小限制层
pub fn body_limit_layer() -> DefaultBodyLimit {
    DefaultBodyLimit::max(250 * MB)
}