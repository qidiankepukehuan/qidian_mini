use crate::response::ApiResponse;
use axum::http::StatusCode;
use axum::{Extension, Json};

use crate::config::AppConfig;
use crate::handler::auth::verify_code;
use crate::middleware::request_id::RequestId;
use crate::utils::email::{Mailer, SmtpMailer};
use crate::utils::github::Submission;
use crate::utils::picture::Base64Image;
use axum_macros::debug_handler;
use serde::Deserialize;
use tracing::{error, info, instrument, warn};
use crate::middleware::background::send_mail_background;

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
#[instrument(
    name = "submit_article_handler",
    skip(payload),
    fields(
        module     = "submit",
        request_id = %request_id,
        email      = %payload.email,
        author     = %payload.author,
        title      = %payload.title,
    )
)]
pub async fn submit_article(
    Extension(RequestId(request_id)): Extension<RequestId>,
    Json(payload): Json<SubmissionRequest>,
) -> ApiResponse<()> {
    info!("SUBMIT_ARTICLE: request received");

    // 先校验验证码
    if !verify_code(payload.email.clone(), payload.email_code.clone()) {
        warn!("SUBMIT_ARTICLE: verify_code failed");
        return ApiResponse::error(
            StatusCode::UNAUTHORIZED,
            "验证码错误或已过期",
            request_id.into(),
        );
    }
    info!("SUBMIT_ARTICLE: verify_code success");

    let mailer = SmtpMailer::global();

    if payload.title.trim() == "测试" && payload.author.trim() == "测试" {
        info!(
            "SUBMIT_ARTICLE: test submission shortcut, email={}",
            payload.email
        );
        // 给提交人发一封“测试通过”邮件
        if let Err(e) = mailer.send(
            &payload.email,
            "投稿测试：已通过",
            "测试通过：系统已成功接收测试提交（未执行真实创建分支/PR/发图等逻辑）。",
        ) {
            warn!(
                "SUBMIT_ARTICLE: test mail send failed for {}: {:#}",
                payload.email, e
            );
        }
        return ApiResponse::success(());
    }

    // 构造 Submission
    let submission = Submission::from_request(payload);
    info!(
        "SUBMIT_ARTICLE: submission built, email={}, title={}",
        submission.email, submission.title,
    );

    // 调用同步 push_branch
    if let Err(e) = submission.push_branch().await {
        error!("SUBMIT_ARTICLE: push_branch failed: {:#}", e);
        return ApiResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("推送分支失败: {}", e),
            request_id.into(),
        );
    }
    info!("SUBMIT_ARTICLE: push_branch success");

    let url = match submission.pull_request().await {
        Ok(url) => {
            info!("SUBMIT_ARTICLE: pull_request created: {}", url);
            url
        }
        Err(e) => {
            error!("SUBMIT_ARTICLE: pull_request failed: {:#}", e);
            return ApiResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("提交失败: {}", e),
                request_id.into(),
            );
        }
    };

    if let Err(e) = mailer.send(
        &submission.email,
        &submission.to_title(),
        &submission.to_contributor(&url),
    ) {
        warn!(
            "SUBMIT_ARTICLE: mail to contributor {} failed: {:#}",
            submission.email, e
        );
    } else {
        info!(
            "SUBMIT_ARTICLE: mail sent to contributor {}",
            submission.email
        );
    }

    let admin_emails = AppConfig::global().admin.email.clone();
    for admin_email in admin_emails {
        send_mail_background(
            mailer.clone(),
            admin_email.clone(), 
            submission.to_title(), 
            submission.to_info()
        );
    }

    info!("SUBMIT_ARTICLE: completed");
    ApiResponse::success(())
}
