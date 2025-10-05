use crate::response::ApiResponse;
use axum::http::StatusCode;
use axum::Json;

use crate::config::AppConfig;
use crate::middleware::mem_map::MemMap;
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
    let cache = MemMap::global();

    // 先校验验证码
    match cache.get::<String>(&payload.email) {
        Some(code) if code == payload.email_code => {
            // 验证成功，删除验证码，防止重放
            cache.remove(&payload.email);
        }
        _ => {
            return ApiResponse::error(
                StatusCode::UNAUTHORIZED,
                "验证码错误或已过期",
            );
        }
    }

    // 构造 Submission
    let submission = Submission::from_request(payload);

    // 调用同步 pull_request
    submission.push_branch().unwrap();
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