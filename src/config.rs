use config::{Config, File};
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use secrecy::{ExposeSecret, SecretBox};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

// 全局配置实例
static CONFIG: OnceCell<AppConfig> = OnceCell::new();

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub github: GitHubConfig,
    pub smtp: SmtpConfig,
    pub admin: AdminConfig,
    pub file_share: FileShareConfig,
}

#[derive(Debug, Deserialize)]
pub struct GitHubConfig {
    pub client_id: SecretBox<String>,
    pub client_secret: SecretBox<String>,
    pub personal_access_token: SecretBox<String>,
    pub redirect_uri: String,
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
pub struct SmtpConfig {
    pub username: String,
    pub password: SecretBox<String>,
    pub host: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminConfig {
    pub email: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileShareConfig {
    pub path: PathBuf,
}

impl AppConfig {
    fn load_config() -> Result<Self, Box<dyn std::error::Error>> {
        // 确保 .env 文件已加载
        dotenv().ok();

        let config = Config::builder()
            .add_source(File::with_name("config.toml").required(false))
            .set_default("app.port", "4052")?
            .set_default("github.client_id", "")?
            .set_default("github.client_secret", "")?
            .set_default("github.redirect_uri", "https://contribute.qidian.space")?
            .set_default(
                "github.repo_path",
                "https://github.com/qidiankepukehuan/qidiankepukehuan",
            )?
            .set_default("smtp.username", "tsblydyzbjb@qidian.space")?
            .set_default("smtp.host", "smtp.163.com")?
            .set_default("admin.emails", vec!["tsblydyzbjb@qidian.space".to_string()])?
            .set_default("file.share_path", "./shared")?
            .build()?;

        // 尝试从不同前缀的环境变量加载
        let github_client_id = env::var("QIDIAN_MINI_GITHUB_CLIENT_ID")
            .or_else(|_| env::var("GITHUB_CLIENT_ID"))
            .map_err(|_| "Neither QIDIAN_MINI_GITHUB_CLIENT_ID nor GITHUB_CLIENT_ID found in environment")?;

        let github_client_secret = env::var("QIDIAN_MINI_GITHUB_CLIENT_SECRET")
            .or_else(|_| env::var("GITHUB_CLIENT_SECRET"))
            .map_err(|_| "Neither QIDIAN_MINI_GITHUB_CLIENT_SECRET nor GITHUB_CLIENT_SECRET found in environment")?;

        let github_personal_access_token = env::var("QIDIAN_MINI_GITHUB_PAT")
            .or_else(|_| env::var("GITHUB_PAT"))
            .map_err(|_| "Neither QIDIAN_MINI_GITHUB_PAT nor GITHUB_PAT found in environment")?;

        let smtp_password = env::var("QIDIAN_MINI_SMTP_PASSWORD")
            .or_else(|_| env::var("SMTP_PASSWORD"))
            .map_err(
                |_| "Neither QIDIAN_MINI_SMTP_PASSWORD nor SMTP_PASSWORD found in environment",
            )?;

        Ok(Self {
            port: config.get::<u16>("app.port")?,
            github: GitHubConfig {
                client_id: SecretBox::new(Box::new(github_client_id)),
                client_secret: SecretBox::new(Box::new(github_client_secret)),
                personal_access_token: SecretBox::new(Box::new(github_personal_access_token)),
                redirect_uri: config.get::<String>("github.redirect_uri")?,
                repo_path: config.get::<String>("github.repo_path")?,
            },
            smtp: SmtpConfig {
                username: config.get::<String>("smtp.username")?,
                password: SecretBox::new(Box::new(smtp_password)),
                host: config.get::<String>("smtp.host")?,
            },
            admin: AdminConfig {
                email: config.get::<Vec<String>>("admin.emails")?,
            },
            file_share: FileShareConfig {
                path: config.get::<PathBuf>("file.share_path")?,
            },
        })
    }

    /// 获取全局配置实例
    pub fn global() -> &'static Self {
        CONFIG.get_or_init(|| Self::load_config().expect("Failed to load config"))
    }
}

impl AppConfig {
    pub fn stats(&self) -> (usize, usize) {
        let checks = [
            !self.github.client_id.expose_secret().is_empty(),
            !self.github.client_secret.expose_secret().is_empty(),
            !self.github.redirect_uri.is_empty(),
            !self.github.repo_path.is_empty(),
            !self.smtp.username.is_empty() && !self.smtp.password.expose_secret().is_empty(),
            !self.smtp.host.is_empty(),
            !self.admin.email.is_empty(),
            !self.file_share.path.as_os_str().is_empty(),
        ];

        let ok = checks.iter().filter(|&&c| c).count();
        let total = checks.len();
        (ok, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// 设置测试环境变量
    fn set_test_env() {
        unsafe {
            env::set_var("QIDIAN_MINI_GITHUB_CLIENT_ID", "test_client_id");
        }
        unsafe {
            env::set_var("QIDIAN_MINI_GITHUB_CLIENT_SECRET", "test_client_secret");
        }
        unsafe {
            env::set_var("QIDIAN_MINI_GITHUB_PAT", "test_pat");
        }
        unsafe {
            env::set_var("QIDIAN_MINI_SMTP_PASSWORD", "test_smtp_password");
        }
    }

    #[test]
    fn test_load_config() {
        set_test_env();

        // 加载全局配置
        let config = AppConfig::load_config().expect("Failed to load config");

        // 验证 github 配置
        assert_eq!(
            config.github.client_id.expose_secret().as_str(),
            "test_client_id"
        );
        assert_eq!(
            config.github.client_secret.expose_secret().as_str(),
            "test_client_secret"
        );
        assert_eq!(
            config.github.personal_access_token.expose_secret().as_str(),
            "test_pat"
        );
        assert!(!config.github.redirect_uri.is_empty());
        assert!(!config.github.repo_path.is_empty());

        // 验证 smtp 配置
        assert_eq!(
            config.smtp.password.expose_secret().as_str(),
            "test_smtp_password"
        );
        assert!(!config.smtp.username.is_empty());
        assert!(!config.smtp.host.is_empty());

        // 验证 admin 配置
        assert!(!config.admin.email.is_empty());
        // 验证路径是否存在
        assert!(config.file_share.path.exists());

        // 验证 stats 方法
        let (ok, total) = config.stats();
        assert_eq!(ok, total);
    }

    #[test]
    fn test_global_config_singleton() {
        set_test_env();

        let global1 = AppConfig::global();
        let global2 = AppConfig::global();

        // 应该是同一个实例
        assert_eq!(global1 as *const _, global2 as *const _);
    }
}
