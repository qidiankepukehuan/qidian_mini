use crate::config::AppConfig;
use anyhow::{Context, Result};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use std::sync::Arc;

pub trait Mailer: Send + Sync {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<()>;

    fn send_code(&self, to: &str, code: &str) -> Result<()> {
        let subject = "您的验证码";
        let body = format!("您的验证码是：{}\n有效期 5 分钟，请勿泄露。", code);
        self.send(to, subject, &body).context("发送验证码邮件失败")
    }
}

pub struct SmtpMailer {
    transport: SmtpTransport,
    from: String,
}

impl SmtpMailer {
    fn new() -> Result<Self> {
        let cfg = AppConfig::global();

        let creds = Credentials::new(
            cfg.smtp.username.clone(),
            cfg.smtp.password.expose_secret().to_string(),
        );

        let transport = SmtpTransport::relay(&cfg.smtp.host)
            .with_context(|| format!("SMTP 服务器地址无效: {}", cfg.smtp.host))?
            .credentials(creds)
            .build();

        Ok(Self {
            transport,
            from: cfg.smtp.username.clone(),
        })
    }

    /// 获取全局单例
    pub fn global() -> Arc<Self> {
        static INSTANCE: Lazy<Arc<SmtpMailer>> =
            Lazy::new(|| Arc::new(SmtpMailer::new().expect("初始化 SMTP Mailer 失败")));
        INSTANCE.clone()
    }
}

impl Mailer for SmtpMailer {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        let from_mailbox = self
            .from
            .parse::<Mailbox>()
            .with_context(|| format!("发件人邮箱地址无效: {}", self.from))?;

        let to_mailbox = to
            .parse::<Mailbox>()
            .with_context(|| format!("收件人邮箱地址无效: {}", to))?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .body(body.to_string())
            .context("构建邮件消息失败")?;

        self.transport
            .send(&email)
            .with_context(|| format!("发送邮件至 {} 失败", to))?;

        Ok(())
    }
}
