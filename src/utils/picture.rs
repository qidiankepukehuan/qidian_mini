use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use std::path::Path;
use serde::{Deserialize, Serialize};

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
    pub fn to_decode_image(&self) -> DecodedImage {
        decode_base64_image(self).unwrap()
    }
    /// 解码成原始字节（支持带 data: 前缀的 base64）
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let s = self.base64.as_str();
        let raw = s.split(',').next_back().unwrap_or(s);
        general_purpose::STANDARD
            .decode(raw)
            .map_err(|e| format!("Base64解码失败 ({}): {}", self.name, e))
    }
    pub fn save(&self, path: &Path)->Result<(),String>{
        self.to_decode_image().save(path)
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
    pub fn save(&self, output_path: &Path) -> Result<(), String> {
        // 创建输出目录（如果不存在）
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }

        // 创建文件并保存图像
        let mut output_file = std::fs::File::create(output_path)
            .map_err(|e| format!("创建文件失败: {}", e))?;

        self.image.write_to(&mut output_file, self.format)
            .map_err(|e| format!("保存图像失败: {}", e))?;

        Ok(())
    }
}

/// 将Base64图像请求解码为图像对象
pub fn decode_base64_image(request: &Base64Image) -> Result<DecodedImage, String> {
    // 解码Base64字符串
    let bytes = general_purpose::STANDARD
        .decode(request.base64.clone())
        .map_err(|e| format!("Base64解码失败 ({}): {}", request.name, e))?;

    // 从文件名推断图像格式
    let format = ImageFormat::from_path(request.name.clone())
        .map_err(|_| format!("无法从文件名推断图像格式: {}", request.name))?;

    // 将字节数据转换为图像对象
    let image = image::load_from_memory_with_format(&bytes, format)
        .map_err(|e| format!("图像解析失败 ({}): {}", request.name, e))?;

    Ok(DecodedImage { image, format })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    // 1x1 像素的 PNG 图像（白色）
    const TEST_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVQIW2NgAAIAAAUAAR4f7BQAAAAASUVORK5CYII=";

    // 1x1 像素的 JPEG 图像（白色）
    const TEST_JPEG_BASE64: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAMCAgICAgMCAgIDAwMDBAYEBAQEBAgGBgUGCQgKCgkICQkKDA8MCgsOCwkJDRENDg8QEBEQCgwSExIQEw8QEBD/2wBDAQMDAwQDBAgEBAgQCwkLEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBD/wAARCAABAAEDAREAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwD9U6AP/9k=";

    // 1x1 像素的 WEBP 图像（白色）
    const TEST_WEBP_BASE64: &str = "UklGRh4AAABXRUJQVlA4TBEAAAAvAAAAAAfQ//73v/+BiOh/AAA=";

    #[test]
    fn test_decode_png() {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.png".to_string());
        let result = decode_base64_image(&request);

        assert!(result.is_ok(), "PNG 解码失败: {:?}", result.err());

        let decoded = result.unwrap();
        assert_eq!(decoded.image.width(), 1);
        assert_eq!(decoded.image.height(), 1);
        assert_eq!(decoded.format, ImageFormat::Png);
    }

    #[test]
    fn test_decode_jpeg() {
        let request = Base64Image::new(TEST_JPEG_BASE64.to_string(), "test.jpg".to_string());
        let result = decode_base64_image(&request);

        assert!(result.is_ok(), "JPEG 解码失败: {:?}", result.err());

        let decoded = result.unwrap();
        assert_eq!(decoded.image.width(), 1);
        assert_eq!(decoded.image.height(), 1);
        assert_eq!(decoded.format, ImageFormat::Jpeg);
    }

    #[test]
    fn test_decode_webp() {
        let request = Base64Image::new(TEST_WEBP_BASE64.to_string(), "test.webp".to_string());
        let result = decode_base64_image(&request);

        assert!(result.is_ok(), "WebP 解码失败: {:?}", result.err());

        let decoded = result.unwrap();
        assert_eq!(decoded.image.width(), 1);
        assert_eq!(decoded.image.height(), 1);
        assert_eq!(decoded.format, ImageFormat::WebP);
    }

    #[test]
    fn test_save_image() {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.png".to_string());
        let decoded = decode_base64_image(&request).unwrap();

        // 创建临时文件
        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path();

        // 保存图像
        let save_result = decoded.save(output_path);
        assert!(save_result.is_ok(), "保存失败: {:?}", save_result.err());

        // 验证文件存在且非空
        assert!(output_path.exists());
        let metadata = std::fs::metadata(output_path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_invalid_base64() {
        let request = Base64Image::new("这不是有效的Base64数据".to_string(), "test.png".to_string());
        let result = decode_base64_image(&request);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Base64解码失败"));
        assert!(error.contains("test.png"));
    }

    #[test]
    fn test_invalid_image_data() {
        // 使用有效的Base64但无效的图像数据
        let request = Base64Image::new("SGVsbG8gV29ybGQh".to_string(), "test.png".to_string()); // "Hello World!"的Base64
        let result = decode_base64_image(&request);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("图像解析失败"));
        assert!(error.contains("test.png"));
    }

    #[test]
    fn test_unknown_format() {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.unknown".to_string());
        let result = decode_base64_image(&request);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("无法从文件名推断图像格式"));
        assert!(error.contains("test.unknown"));
    }

    #[test]
    fn test_create_parent_directories() {
        let request = Base64Image::new(TEST_PNG_BASE64.to_string(), "test.png".to_string());
        let decoded = decode_base64_image(&request).unwrap();

        // 创建临时目录结构
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("subdir/test.png");

        // 保存图像（应自动创建subdir目录）
        let save_result = decoded.save(&output_path);
        assert!(save_result.is_ok(), "保存失败: {:?}", save_result.err());

        // 验证目录和文件存在
        assert!(output_path.parent().unwrap().exists());
        assert!(output_path.exists());
    }
}