use crate::config::AppConfig;
use crate::middleware::mem_map::{MemMap, ToKey};
use crate::to_key;

use crate::utils::stream::file_stream_with_md5;
use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use chrono::{Duration, Utc};
use futures_util::Stream;
use reqwest::{Body, Client, multipart};
use serde::{Deserialize, Serialize};
use tokio::{fs, io};
use tracing::{debug, info, warn, error, instrument};

// 缓存时间常量
// 列表10分钟更新
const LIST_TTL: Duration = Duration::minutes(10);
// 文件3天更新
const FILE_TTL: Duration = Duration::days(3);

fn validate_filename_only(input: &str) -> Result<String, &'static str> {
    let s = input.trim();
    if s.is_empty() {
        return Err("非法文件名：为空");
    }
    if s.contains('/') || s.contains('\\') {
        return Err("非法文件名：不允许路径分隔符");
    }
    if s.contains("..") {
        return Err("非法文件名：不允许 ..");
    }
    // 可按需要决定是否允许以 '.' 开头（隐藏文件）
    if s.starts_with('.') || s.starts_with('-') {
        return Err("非法文件名：不允许以 . 或 - 开头");
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
    if !s.chars().all(allowed) {
        return Err("非法文件名：包含未允许的字符");
    }
    Ok(s.to_string())
}

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
    /// 从缓存或本地文件读取元数据
    #[instrument(
        name = "sharefile_get",
        fields(
            module = "sharefile",
            file   = %file_name,
        )
    )]
    pub async fn get(file_name: &str) -> Result<Self> {
        let allowed = Self::list().await?;
        if !allowed.contains(&file_name.to_string()) {
            warn!("SHAREFILE_GET: illegal file request: {}", file_name);
            return Err(anyhow!(
                "非法文件：请选择 /list_files 返回的文件之一（收到：{}）",
                file_name
            ));
        }

        let safe_name =
            validate_filename_only(file_name).map_err(|msg| anyhow::anyhow!(msg))?;

        // 检查缓存
        let cache = MemMap::global();
        let file_key = ShareFileKey::new(&safe_name);
        if let Some(v) = cache.get::<ShareFileKey, ShareFile>(&file_key) {
            debug!("SHAREFILE_GET: cache hit for {}", safe_name);
            return Ok(v);
        }
        debug!("SHAREFILE_GET: cache miss for {}, reading from disk", safe_name);

        let config = AppConfig::global();
        let file_path = config.file_share.path.join(&safe_name);

        // 文件是否存在
        if !file_path.exists() {
            error!("SHAREFILE_GET: file not found: {}", file_path.display());
            return Err(anyhow!("文件不存在: {}", file_path.display()));
        }

        // 1. 构造“带 md5 副作用”的流
        let (stream, md5_handle) = file_stream_with_md5(&file_path).await?;
        debug!("SHAREFILE_GET: stream with md5 created for {}", safe_name);

        // 2. 流式上传
        let upload_info = Self::upload_stream_to_tmpfile(&safe_name, stream).await?;
        info!(
            "SHAREFILE_GET: upload completed, file={}, size={}",
            upload_info.file_name, upload_info.size
        );

        // 3. 上传结束后再 finalize md5
        let md5 = md5_handle.finalize()?;
        debug!(%md5, "SHAREFILE_GET: md5 finalized");

        let share_file = ShareFile {
            file_name: safe_name.to_string(),
            timestamp: Utc::now().timestamp(),
            download_link: upload_info.download_link,
            download_link_encoded: upload_info.download_link_encoded,
            size: upload_info.size,
            mime_type: upload_info.mime_type,
            md5,
        };

        // 更新到cache
        cache.insert(file_key, share_file.clone(), FILE_TTL);
        debug!("SHAREFILE_GET: cache updated for {}", share_file.file_name);

        Ok(share_file)
    }

    /// 通过任意字节流上传到 tmpfile.link（流式）
    #[instrument(
        name = "sharefile_upload_stream",
        skip(stream),
        fields(
            module   = "sharefile",
            filename = %filename,
        )
    )]
    pub async fn upload_stream_to_tmpfile<S>(
        filename: &str,
        stream: S,
    ) -> Result<TmpfileResponse>
    where
        S: Stream<Item = Result<Bytes, io::Error>> + Send + 'static,
    {
        debug!("SHAREFILE_UPLOAD: building request body");

        // 用 stream 构造 reqwest Body
        let body = Body::wrap_stream(stream);

        // multipart 的 file part 使用 stream
        let part = multipart::Part::stream(body)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")?;

        let form = multipart::Form::new().part("file", part);
        let client = Client::new();

        debug!("SHAREFILE_UPLOAD: sending request to tmpfile.link");
        let resp = client
            .post("https://tmpfile.link/api/upload")
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        let tmp_resp = resp.json::<TmpfileResponse>().await?;
        info!(
            "SHAREFILE_UPLOAD: upload finished, remote_file={}, size={}",
            tmp_resp.file_name, tmp_resp.size
        );

        Ok(tmp_resp)
    }

    /// 获取文件列表（带缓存）
    #[instrument(
        name = "sharefile_list",
        fields(module = "sharefile")
    )]
    pub async fn list() -> Result<Vec<String>> {
        let cache = MemMap::global();
        let list_key = ShareFileListKey::new();

        if let Some(v) = cache.get::<ShareFileListKey, Vec<String>>(&list_key) {
            debug!("SHAREFILE_LIST: cache hit, count={}", v.len());
            return Ok(v);
        }
        debug!("SHAREFILE_LIST: cache miss, reading directory");

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

        debug!(
            "SHAREFILE_LIST: directory scan finished, count={}",
            file_names.len()
        );

        // 更新缓存
        cache.insert(list_key, file_names.clone(), LIST_TTL);
        debug!("SHAREFILE_LIST: cache updated");

        Ok(file_names)
    }
}
