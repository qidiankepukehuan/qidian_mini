use crate::config::AppConfig;
use crate::handler::auth::verify_code;
use crate::response::ApiResponse;
use crate::utils::email::{Mailer, SmtpMailer};
use crate::utils::file::ShareFile;
use anyhow::Context;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Local, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ShareRequest {
    pub applicant: String,
    pub apply_for: String,
    pub email: String,
    pub email_code: String,
}

pub async fn share_files(Json(payload): Json<ShareRequest>) -> ApiResponse<()> {
    // 校验验证码
    if !verify_code(payload.email.clone(), payload.email_code.clone()) {
        return ApiResponse::error(StatusCode::UNAUTHORIZED, "验证码错误或已过期");
    }

    // 获取文件（缓存 + 上传 tmpfile.link）
    let file = match ShareFile::get(&payload.apply_for).await {
        Ok(file) => file,
        Err(e) => {
            return ApiResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("获取文件失败: {:#}", e),
            );
        }
    };

    // 将时间戳转为可读时间
    let formatted_time = DateTime::<Utc>::from_timestamp(file.timestamp, 0)
        .map(|utc_time| {
            utc_time
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| format!("无效时间戳: {}", file.timestamp));

    // 邮件构造
    let subject_user = format!("文件分享通知 - {}", file.file_name);
    let body_user = format!(
        "尊敬的 {}，您好：\n\n\
        您申请的文件已准备就绪，可通过以下链接下载：\n\n\
        下载地址：{}\n\
        文件名：{}\n\
        文件大小：{} 字节\n\
        生成时间：{}\n\n\
        链接有效期为 24 小时，请尽快下载。\n\n\
        —— 系统自动发送，请勿回复。",
        payload.applicant, file.download_link, file.file_name, file.size, formatted_time,
    );

    let mailer = SmtpMailer::global();

    // 发给用户
    if let Err(e) = mailer
        .send(&payload.email, &subject_user, &body_user)
        .context("发送文件通知邮件失败")
    {
        return ApiResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("邮件发送失败: {:#}", e),
        );
    }

    // 通知管理员（不会阻断主流程）
    let admin_emails = AppConfig::global().admin.email.clone();
    let subject_admin = format!("用户申请文件下载 - {}", payload.applicant);
    let body_admin = format!(
        "用户 {} ({}) 申请下载文件：{}\n\
        下载链接：{}\n\
        生成时间：{}\n",
        payload.applicant, payload.email, file.file_name, file.download_link, formatted_time,
    );

    for admin_email in admin_emails {
        if let Err(e) = mailer.send(&admin_email, &subject_admin, &body_admin) {
            eprintln!("发送邮件给管理员 {} 失败: {}", admin_email, e);
        }
    }

    ApiResponse::success(())
}

pub async fn list_files() -> ApiResponse<Vec<String>> {
    match ShareFile::list().await {
        Ok(files) => ApiResponse::success(files),
        Err(e) => ApiResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("读取文件列表失败: {:#}", e),
        ),
    }
}
