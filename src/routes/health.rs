use axum::{
    routing::get,
    Router,
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use crate::config::AppConfig;
use crate::response::ApiResponse;

#[derive(Deserialize, Serialize)]
pub struct Health{
    config: String,
    github: String,
}

pub fn routes() -> Router {
    Router::new().route("/health", get(health))
}


async fn health() -> ApiResponse<Health> {
    let config = AppConfig::global();
    let (config_ok, config_total) = config.stats();

    // GitHub 连通性检测
    let github_status = match check_github().await {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("error: {}", e),
    };

    ApiResponse::success(Health {
        config: format!("{}/{}", config_ok, config_total),
        github: github_status,
    })
}

async fn check_github() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::global();
    let token = config.github.personal_access_token.expose_secret();

    let client = reqwest::Client::new();
    let res = client
        .get("https://api.github.com/rate_limit")
        .header("User-Agent", "qidian-healthcheck")
        .bearer_auth(token) // 用 PAT
        .send()
        .await?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("GitHub returned status {}", res.status()).into())
    }
}