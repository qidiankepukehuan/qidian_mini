use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// 通用响应结构
#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            message: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(status: StatusCode, message: &str) -> Self {
        Self {
            code: status.as_u16(),
            message: message.to_string(),
            data: None,
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::OK);
        let body = axum::Json(self);
        (status, body).into_response()
    }
}
