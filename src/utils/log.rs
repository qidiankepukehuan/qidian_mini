use std::fs;
use std::sync::OnceLock;

use tracing_subscriber::{fmt, EnvFilter};

use crate::config::{AppConfig, LogFormat};

static GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// 初始化全局 tracing（在 main() 里调用一次）
pub fn init_tracing() {
    let cfg = AppConfig::global();
    let log_cfg = &cfg.log;

    // 1. 确保日志目录存在
    if let Err(e) = fs::create_dir_all(&log_cfg.dir) {
        eprintln!("Failed to create log directory {:?}: {e}", log_cfg.dir);
    }

    // 2. 计算日志文件路径：/var/log/qidian/{level}.log
    let log_file_path = log_cfg.file_for_level(log_cfg.level);

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .unwrap_or_else(|e| {
            panic!("Failed to open log file {:?}: {e}", log_file_path);
        });

    // non_blocking writer + guard
    let (non_blocking, guard) = tracing_appender::non_blocking(file);
    let _ = GUARD.set(guard);

    // 3. EnvFilter：优先用 RUST_LOG，其次用配置里的 level
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_cfg.level.as_str()))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 4. 根据不同 format 构建不同的 subscriber
    match log_cfg.format {
        LogFormat::Text => {
            fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .with_writer(non_blocking)
                .init();
        }
        LogFormat::Compact => {
            fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .compact()
                .with_writer(non_blocking)
                .init();
        }
        LogFormat::Json => {
            fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .json()
                .with_writer(non_blocking)
                .init();
        }
    }

    tracing::info!(
        level = %log_cfg.level,
        format = %log_cfg.format,
        dir = ?log_cfg.dir,
        file = ?log_file_path,
        "tracing initialized",
    );
}