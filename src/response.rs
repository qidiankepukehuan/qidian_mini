use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use crate::middleware::request_id::RequestId;

/// 通用响应结构
#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
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
            request_id: None,
        }
    }

    pub fn error(status: StatusCode, message: &str, request_id: RequestId) -> Self {
        Self {
            code: status.as_u16(),
            message: message.to_string(),
            data: None,
            request_id: Some(request_id.to_string()),
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
