use crate::response::ApiResponse;
use axum::Json;
use axum::http::StatusCode;

use crate::config::AppConfig;
use crate::handler::auth::verify_code;
use crate::utils::email::{Mailer, SmtpMailer};
use crate::utils::github::Submission;
use crate::utils::picture::Base64Image;
use axum_macros::debug_handler;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SubmissionRequest {
    pub author: String,
    pub content: String,
    pub cover: Base64Image,
    pub email: String,
    pub email_code: String,
    pub images: Vec<Base64Image>,
    pub tags: Vec<String>,
    pub title: String,
}

#[debug_handler]
pub async fn submit_article(Json(payload): Json<SubmissionRequest>) -> ApiResponse<()> {
    // 先校验验证码
    if !verify_code(payload.email.clone(), payload.email_code.clone()) {
        return ApiResponse::error(StatusCode::UNAUTHORIZED, "验证码错误或已过期");
    }

    // 构造 Submission
    let submission = Submission::from_request(payload);

    // 调用同步 pull_request
    if let Err(e) = submission.push_branch().await {
        return ApiResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("推送分支失败: {}", e),
        );
    }

    if let Err(e) = submission.pull_request().await {
        return ApiResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("提交失败: {}", e),
        );
    }

    let emails = AppConfig::global().admin.email.clone();
    let mailer = SmtpMailer::global();

    for email in emails {
        if let Err(e) = mailer.send(&email, &submission.to_title(), &submission.to_info()) {
            eprintln!("发送邮件给 {} 失败: {}", email, e);
        }
    }

    ApiResponse::success(())
}
