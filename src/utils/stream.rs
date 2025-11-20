use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::TryStreamExt;
use md5::{Digest, Md5};
use tokio::fs::File;
use tokio::io;
use tokio_util::io::ReaderStream;
use tracing::{debug, trace, warn, instrument};

#[derive(Clone)]
pub struct Md5Handle(Arc<Mutex<Option<Md5>>>);

impl Md5Handle {
    #[instrument(
        name = "md5_finalize",
        skip(self),
        level = "debug"
    )]
    pub fn finalize(self) -> Result<String> {
        use std::mem;

        let mut guard = self
            .0
            .lock()
            .map_err(|_| anyhow!("md5 hasher poisoned"))?;

        let hasher = mem::take(&mut *guard)
            .ok_or_else(|| anyhow!("md5 already finalized or never initialized"))?;

        let hex = format!("{:x}", hasher.finalize());
        debug!(md5 = %hex, "MD5_HANDLE: finalize success");
        Ok(hex)
    }
}

#[instrument(
    name = "with_md5",
    skip(stream),
    level = "debug"
)]
pub fn with_md5<S>(
    stream: S,
) -> (
    impl futures_util::Stream<Item = Result<Bytes, io::Error>> + Send + 'static,
    Md5Handle,
)
where
    S: futures_util::Stream<Item = Result<Bytes, io::Error>> + Send + 'static,
{
    let handle = Md5Handle(Arc::new(Mutex::new(Some(Md5::new()))));
    debug!("MD5_HANDLE: created new hasher");

    let handle_clone = handle.clone();

    let wrapped = stream.inspect_ok(move |chunk: &Bytes| {
        // 这里会被调用很多次，用 trace，避免 info/warn 把日志刷爆
        match handle_clone.0.lock() {
            Ok(mut guard) => {
                if let Some(ref mut h) = *guard {
                    h.update(&chunk[..]);
                    trace!(
                        chunk_len = chunk.len(),
                        "MD5_HANDLE: updated hasher with chunk"
                    );
                } else {
                    // 一般不会出现，出现说明 finalize 过早调用
                    warn!(
                        chunk_len = chunk.len(),
                        "MD5_HANDLE: hasher already taken (md5 may be incomplete)"
                    );
                }
            }
            Err(_) => {
                warn!(
                    chunk_len = chunk.len(),
                    "MD5_HANDLE: mutex poisoned, md5 will be invalid"
                );
            }
        }
    });

    (wrapped, handle)
}

#[instrument(
    name = "file_stream_with_md5",
    skip(path),
    fields(path = %path.display()),
    level = "debug"
)]
pub async fn file_stream_with_md5(
    path: &PathBuf,
) -> Result<(
    impl futures_util::Stream<Item = Result<Bytes, io::Error>> + Send + 'static,
    Md5Handle,
)> {
    debug!("MD5_FILE_STREAM: opening file");
    let file = File::open(path).await?;
    let stream = ReaderStream::new(file);
    let (wrapped, handle) = with_md5(stream);
    debug!("MD5_FILE_STREAM: stream + md5 wrapper created");
    Ok((wrapped, handle))
}