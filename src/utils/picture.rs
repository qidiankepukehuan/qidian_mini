use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use image::{DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 表示一个Base64编码的图像请求
#[derive(Deserialize, Serialize)]
pub struct Base64Image {
    pub base64: String,
    pub name: String,
}

impl Base64Image {
    /// 创建新的Base64图像请求
    pub fn new(base64_str: String, image_name: String) -> Self {
        Self {
            base64: base64_str,
            name: image_name,
        }
    }
    pub fn to_decode_image(&self) -> Result<DecodedImage> {
        decode_base64_image(self).context("解码 Base64 图像失败")
    }
    /// 解码成原始字节（支持带 data: 前缀的 base64）
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let s = self.base64.as_str();
        let raw = s.split(',').next_back().unwrap_or(s);

        general_purpose::STANDARD
            .decode(raw)
            .map_err(|e| anyhow!("Base64 解码失败 ({}): {}", self.name, e))
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.to_decode_image()
            .context("图像解码失败")?
            .save(path)
            .context("图像保存失败")
    }
}

/// 表示解码后的图像对象及其格式
#[derive(Debug)]
pub struct DecodedImage {
    pub image: DynamicImage,
    pub format: ImageFormat,
}

impl DecodedImage {
    /// 将图像保存到指定路径
    pub fn save(&self, output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建目录失败: {}", parent.display()))?;
        }

        let mut output_file = std::fs::File::create(output_path)
            .with_context(|| format!("创建文件失败: {}", output_path.display()))?;

        self.image
            .write_to(&mut output_file, self.format)
            .with_context(|| format!("保存图像失败: {}", output_path.display()))?;

        Ok(())
    }
}

/// 将Base64图像请求解码为图像对象
pub fn decode_base64_image(request: &Base64Image) -> Result<DecodedImage> {
    // 解码 Base64
    let bytes = general_purpose::STANDARD
        .decode(&request.base64)
        .with_context(|| format!("Base64解码失败 ({})", request.name))?;

    // 从文件名推断图像格式
    let format = ImageFormat::from_path(&request.name)
        .with_context(|| format!("无法从文件名推断图像格式: {}", request.name))?;

    // 加载图像
    let image = image::load_from_memory_with_format(&bytes, format)
        .with_context(|| format!("图像解析失败 ({})", request.name))?;

    Ok(DecodedImage { image, format })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    const TEST_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVQIW2NgAAIAAAUAAR4f7BQAAAAASUVORK5CYII=";
    const TEST_JPEG_BASE64: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAMCAgICAgMCAgIDAwMDBAYEBAQEBAgGBgUGCQgKCgkICQkKDA8MCgsOCwkJDRENDg8QEBEQCgwSExIQEw8QEBD/2wBDAQMDAwQDBAgEBAgQCwkLEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBD/wAARCAABAAEDAREAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwD9U6AP/9k=";
    const TEST_WEBP_BASE64: &str = "UklGRh4AAABXRUJQVlA4TBEAAAAvAAAAAAfQ//73v/+BiOh/AAA=";

    #[test]
    fn test_decode_png() -> Result<()> {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.png".to_string());
        let decoded = decode_base64_image(&request)?;
        assert_eq!(decoded.image.width(), 1);
        assert_eq!(decoded.image.height(), 1);
        assert_eq!(decoded.format, ImageFormat::Png);
        Ok(())
    }

    #[test]
    fn test_decode_jpeg() -> Result<()> {
        let request = Base64Image::new(TEST_JPEG_BASE64.to_string(), "test.jpg".to_string());
        let decoded = decode_base64_image(&request)?;
        assert_eq!(decoded.image.width(), 1);
        assert_eq!(decoded.image.height(), 1);
        assert_eq!(decoded.format, ImageFormat::Jpeg);
        Ok(())
    }

    #[test]
    fn test_decode_webp() -> Result<()> {
        let request = Base64Image::new(TEST_WEBP_BASE64.to_string(), "test.webp".to_string());
        let decoded = decode_base64_image(&request)?;
        assert_eq!(decoded.image.width(), 1);
        assert_eq!(decoded.image.height(), 1);
        assert_eq!(decoded.format, ImageFormat::WebP);
        Ok(())
    }

    #[test]
    fn test_save_image() -> Result<()> {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.png".to_string());
        let decoded = decode_base64_image(&request)?;
        let temp_file = NamedTempFile::new()?;
        decoded.save(temp_file.path())?;
        assert!(temp_file.path().exists());
        Ok(())
    }

    #[test]
    fn test_invalid_base64() {
        let request =
            Base64Image::new("这不是有效的Base64数据".to_string(), "test.png".to_string());
        let result = decode_base64_image(&request);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Base64解码失败"));
    }

    #[test]
    fn test_unknown_format() {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.unknown".to_string());
        let result = decode_base64_image(&request);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("无法从文件名推断图像格式")
        );
    }

    #[test]
    fn test_create_parent_directories() -> Result<()> {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.png".to_string());
        let decoded = decode_base64_image(&request)?;
        let temp_dir = tempfile::tempdir()?;
        let output_path = temp_dir.path().join("subdir/test.png");
        decoded.save(&output_path)?;
        assert!(output_path.exists());
        Ok(())
    }
}
