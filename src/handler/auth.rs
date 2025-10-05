use crate::middleware::mem_map::MemMap;
use crate::response::ApiResponse;
use axum::{extract::Json, http::StatusCode};
use rand::distr::Alphanumeric;
use rand::Rng;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use crate::utils::email::{Mailer, SmtpMailer};

#[derive(Deserialize)]
pub struct SendCodeRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct VerifyCodeRequest {
    pub email: String,
    pub code: String,
}

pub async fn do_send_code(
    Json(payload): Json<SendCodeRequest>,
    mailer: Arc<dyn Mailer>,
) -> ApiResponse<String> {
    let cache = MemMap::global();

    // 生成6位验证码
    let code: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    // 存入缓存
    cache.insert(payload.email.clone(), code.clone(), Duration::from_secs(300));

    // 发送验证码
    match mailer.send_code(&payload.email, &code) {
        Ok(_) => ApiResponse::success(format!("验证码已发送到 {}", payload.email)),
        Err(e) => ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, &format!("邮件发送失败: {}", e)),
    }
}

// 发送验证码
pub async fn send_code(Json(payload): Json<SendCodeRequest>) -> ApiResponse<String> {
    let mailer = SmtpMailer::global();
    do_send_code(Json::from(payload), mailer.clone()).await
}

// 验证验证码
pub async fn verify_code(Json(payload): Json<VerifyCodeRequest>) -> ApiResponse<bool> {
    let cache = MemMap::global();

    let valid = matches!(cache.get::<String>(&payload.email), Some(v) if v == payload.code);

    // 匹配成功后可以选择删除缓存
    if valid {
        cache.remove(&payload.email);
    }

    ApiResponse::success(valid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::mem_map::MemMap;
    use axum::extract::Json;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;
    use serde::Deserialize;

    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct MockMailer {
        pub sent: Arc<Mutex<Vec<(String, String, String)>>>,
    }

    impl Mailer for MockMailer {
        fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
            self.sent
                .lock()
                .unwrap()
                .push((to.to_string(), subject.to_string(), body.to_string()));
            Ok(())
        }
    }

    #[derive(Deserialize)]
    struct ApiBoolResponse {
        code: u16,
        message: String,
        data: Option<bool>,
    }

    #[tokio::test]
    async fn test_send_and_verify_code() {
        let email = "test@example.com".to_string();
        let mailer= Arc::new(MockMailer::default());

        // 发送验证码
        let send_req = SendCodeRequest { email: email.clone() };
        let resp = do_send_code(Json(send_req), mailer.clone()).await.into_response();
        let body = resp.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.contains("验证码已发送到"));

        // 确认 MockMailer 收到了邮件
        let sent = mailer.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, email);

        // 从缓存取验证码
        let cache = MemMap::global();
        let code_in_cache = cache.get::<String>(&email).expect("验证码应存在缓存中");

        // 验证正确验证码
        let verify_req = VerifyCodeRequest {
            email: email.clone(),
            code: code_in_cache.clone(),
        };
        let resp = verify_code(Json(verify_req)).await.into_response();
        let body = resp.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let body: ApiBoolResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(body.data.unwrap());

        // 验证码已被删除
        assert!(cache.get::<String>(&email).is_none());
    }
}
