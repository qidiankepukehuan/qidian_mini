use std::path::PathBuf;

use crate::config::AppConfig;
use crate::middleware::mem_map::{MemMap, ToKey};
use crate::to_key;

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, Utc};
use md5::{Digest, Md5};
use reqwest::{Client, multipart};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

// 缓存时间常量
// 列表10分钟更新
const LIST_TTL: Duration = Duration::minutes(10);
// 文件3天更新
const FILE_TTL: Duration = Duration::days(3);

/// 缓存中存储的文件信息
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ShareFile {
    pub file_name: String,
    pub timestamp: i64,
    pub download_link: String,
    pub download_link_encoded: String,
    pub size: u64,
    pub mime_type: String,
    pub md5: String,
}

/// 文件缓存 Key
pub struct ShareFileKey {
    pub module: &'static str,
    pub file_name: String,
}

impl ShareFileKey {
    pub fn new(file_name: &str) -> Self {
        Self {
            module: "ShareFile",
            file_name: file_name.to_string(),
        }
    }
}
to_key!(ShareFileKey; module=module; file_name);

/// 文件列表缓存 Key
pub struct ShareFileListKey {
    pub module: &'static str,
    pub second_module: &'static str,
}

impl ShareFileListKey {
    pub fn new() -> Self {
        Self {
            module: "ShareFile",
            second_module: "List",
        }
    }
}
to_key!(ShareFileListKey; module=module; second_module);

/// tmpfile.link 上传返回结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmpfileResponse {
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "downloadLink")]
    pub download_link: String,
    #[serde(rename = "downloadLinkEncoded")]
    pub download_link_encoded: String,
    pub size: u64,
    #[serde(rename = "type")]
    pub mime_type: String,
    #[serde(rename = "uploadedTo")]
    pub uploaded_to: String,
}

impl ShareFile {
    /// 从缓存或本地文件读取元数据
    pub async fn get(file_name: &str) -> Result<Self> {
        // 检查缓存
        let cache = MemMap::global();
        let file_key = ShareFileKey::new(file_name);
        if let Some(v) = cache.get::<ShareFileKey, ShareFile>(&file_key) {
            return Ok(v);
        }

        let config = AppConfig::global();
        let file_path = config.file_share.path.join(file_name);

        // 文件是否存在
        if !file_path.exists() {
            return Err(anyhow!("文件不存在: {}", file_path.display()));
        }

        // 计算MD5哈希
        // 直接 await，简单直观
        let md5 = Self::get_md5_from_filepath(&file_path).await?;
        // 上传文件
        let upload_info = Self::upload_to_tmpfile(&file_path).await?;

        let share_file = ShareFile {
            file_name: file_name.to_string(),
            timestamp: Utc::now().timestamp(),
            download_link: upload_info.download_link,
            download_link_encoded: upload_info.download_link_encoded,
            size: upload_info.size,
            mime_type: upload_info.mime_type,
            md5,
        };

        // 更新到cache
        cache.insert(file_key, share_file.clone(), FILE_TTL);

        Ok(share_file)
    }

    /// 计算对应文件的md5
    pub async fn get_md5_from_filepath(file_path: &PathBuf) -> Result<String> {
        let mut file = File::open(&file_path).await?;
        let mut hasher = Md5::new();
        let mut buffer = [0; 16 * 1024];
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        let md5 = format!("{:x}", hasher.finalize());
        Ok(md5)
    }

    /// 上传到 tmpfile.link
    pub async fn upload_to_tmpfile(path: &PathBuf) -> Result<TmpfileResponse> {
        // 读取文件内容
        let filename = path
            .file_name()
            .ok_or_else(|| anyhow!("文件名无效"))?
            .to_string_lossy()
            .to_string();
        let bytes = tokio::fs::read(&path).await?;

        // 构造 multipart
        let part = multipart::Part::bytes(bytes)
            .file_name(filename.clone())
            .mime_str("application/octet-stream")?;

        let form = multipart::Form::new().part("file", part);
        let client = Client::new();

        let resp = client
            .post("https://tmpfile.link/api/upload")
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .json::<TmpfileResponse>()
            .await?;

        Ok(resp)
    }

    /// 获取文件列表（带缓存）
    pub async fn list() -> Result<Vec<String>> {
        let cache = MemMap::global();
        let list_key = ShareFileListKey::new();

        if let Some(v) = cache.get::<ShareFileListKey, Vec<String>>(&list_key) {
            return Ok(v);
        }

        let config = AppConfig::global();
        let dir_path = &config.file_share.path;

        let mut entries = fs::read_dir(dir_path)
            .await
            .with_context(|| format!("读取目录失败: {}", dir_path.display()))?;

        let mut file_names = Vec::new();

        while let Some(entry) = entries.next_entry().await.context("读取目录项失败")? {
            let path = entry.path();

            if path.is_file()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                file_names.push(name.to_string());
            }
        }

        // 更新缓存
        cache.insert(list_key, file_names.clone(), LIST_TTL);

        Ok(file_names)
    }
}
