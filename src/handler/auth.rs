use crate::middleware::mem_map::{MemMap, ToKey};
use crate::response::ApiResponse;
use crate::to_key;
use crate::utils::email::{Mailer, SmtpMailer};
use axum::{extract::Json, http::StatusCode};
use rand::Rng;
use rand::distr::Alphanumeric;
use serde::Deserialize;
use std::sync::Arc;
use chrono::Duration;

#[derive(Deserialize)]
pub struct SendCodeRequest {
    pub email: String,
}

pub struct EmailVerifyKey {
    pub module: &'static str,
    pub email: String,
}

impl EmailVerifyKey {
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            module: "email-verify",
            email: email.into(),
        }
    }
}

to_key!(EmailVerifyKey; module=module; email);

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

    // 创建键
    let key = EmailVerifyKey::new(payload.email.clone());

    // 存入缓存
    cache.insert(key, code.clone(), Duration::minutes(5));

    // 发送验证码
    match mailer.send_code(&payload.email, &code) {
        Ok(_) => ApiResponse::success(format!("验证码已发送到 {}", payload.email)),
        Err(e) => ApiResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("邮件发送失败: {}", e),
        ),
    }
}

// 发送验证码
pub async fn send_code(Json(payload): Json<SendCodeRequest>) -> ApiResponse<String> {
    let mailer = SmtpMailer::global();
    do_send_code(Json::from(payload), mailer.clone()).await
}

// 验证验证码
pub fn verify_code(email: String, code: String) -> bool {
    let cache = MemMap::global();

    // 创建键
    let key = EmailVerifyKey::new(email);

    let valid = matches!(cache.get::<EmailVerifyKey, String>(&key), Some(v) if v == code);

    if valid {
        cache.remove(&key);
    }
    valid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::mem_map::MemMap;
    use axum::extract::Json;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct MockMailer {
        pub sent: Arc<Mutex<Vec<(String, String, String)>>>,
    }

    impl Mailer for MockMailer {
        fn send(&self, to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
            self.sent
                .lock()
                .unwrap()
                .push((to.to_string(), subject.to_string(), body.to_string()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_send_and_verify_code() {
        let email = "test@example.com".to_string();
        let mailer = Arc::new(MockMailer::default());

        // 发送验证码
        let send_req = SendCodeRequest {
            email: email.clone(),
        };
        let resp = do_send_code(Json(send_req), mailer.clone())
            .await
            .into_response();
        let body = resp.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.contains("验证码已发送到"));

        // 确认 MockMailer 收到了邮件
        let sent = mailer.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, email);

        // 从缓存取验证码（用与写入一致的 key 结构）
        let cache = MemMap::global();
        let key = EmailVerifyKey {
            module: "email-verify",
            email: email.clone(),
        };
        let code_in_cache = cache
            .get::<EmailVerifyKey, String>(&key)
            .expect("验证码应存在缓存中");

        // 验证正确验证码
        let resp = verify_code(email.clone(), code_in_cache.clone());

        assert!(resp);
        assert!(cache.get::<EmailVerifyKey, String>(&key).is_none());
    }
    #[tokio::test]
    async fn test_key_name() {
        struct TestKey {
            key: String,
            name: String,
            module: String,
        }
        to_key!(TestKey;module=module;key,name);
        let test_key = TestKey {
            key: "key".to_string(),
            name: "name".to_string(),
            module: "module".to_string(),
        };
        assert_eq!(test_key.to_key(), "module@key-name");
        let key = EmailVerifyKey {
            module: "email-verify",
            email: "test@example.com".to_string(),
        };
        assert_eq!(key.to_key(), "email-verify@test@example.com");
    }
}
