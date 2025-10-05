use std::sync::Arc;
use lettre::{Message, SmtpTransport, Transport};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use crate::config::AppConfig;

pub trait Mailer: Send + Sync {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
    fn send_code(&self, to: &str, code: &str) -> Result<(), String> {
        let subject = "您的验证码";
        let body = format!("您的验证码是：{}\n有效期 5 分钟，请勿泄露。", code);
        self.send(to, subject, &body)
    }
}

pub struct SmtpMailer {
    transport: SmtpTransport,
    from: String,
}

impl SmtpMailer {
    fn new() -> Self {
        let cfg = AppConfig::global();

        let creds = Credentials::new(
            cfg.smtp.username.clone(),
            cfg.smtp.password.expose_secret().to_string(),
        );

        let transport = SmtpTransport::relay(&cfg.smtp.host)
            .expect("Invalid SMTP host")
            .credentials(creds)
            .build();

        Self {
            transport,
            from: cfg.smtp.username.clone(),
        }
    }

    /// 获取全局单例
    pub fn global() -> Arc<Self> {
        static INSTANCE: Lazy<Arc<SmtpMailer>> = Lazy::new(|| Arc::new(SmtpMailer::new()));
        INSTANCE.clone()
    }
}

impl Mailer for SmtpMailer {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        let email = Message::builder()
            .from(self.from.parse::<Mailbox>().map_err(|e| e.to_string())?)
            .to(to.parse::<Mailbox>().map_err(|e| e.to_string())?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| e.to_string())?;

        self.transport.send(&email).map_err(|e| e.to_string())?;
        Ok(())
    }
}