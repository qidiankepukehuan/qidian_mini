use chrono::Local;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

#[derive(Clone)]
pub struct Markdown{
    pub author: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

pub trait ToHexo {
    fn to_hexo(&self) -> String;

    fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let mut file = File::create(path)?;
        file.write_all(self.to_hexo().as_bytes())
    }

    fn to_tempfile(&self) -> Result<NamedTempFile, io::Error> {
        let mut tmp = NamedTempFile::new()?;
        tmp.write_all(self.to_hexo().as_bytes())?;
        Ok(tmp)
    }
}

impl ToHexo for Markdown {
    fn to_hexo(&self) -> String {
        // 当前时间作为日期
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 处理 tags
        let tags_yaml = if self.tags.is_empty() {
            "".to_string()
        } else {
            let mut s = String::new();
            for tag in &self.tags {
                s.push_str(&format!("- {}\n", tag));
            }
            s
        };

        // 禁止进行缩进
        format!(
r#"---
title: {title}
author: {author}
date: {date}
tags:
{tags}cover: cover.webp
---
{content}
"#,
            title = self.title,
            author = self.author,
            date = now,
            tags = tags_yaml,
            content = self.content,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::file::{Markdown, ToHexo};
    use chrono::NaiveDateTime;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_to_file_and_tempfile() {
        let md = Markdown {
            author: "Test Author".to_string(),
            title: "Test Title".to_string(),
            tags: vec!["rust".to_string(), "hexo".to_string()],
            content: "This is a test markdown content.".to_string(),
        };

        // ---------- 测试保存到指定文件 ----------
        let path = "test.md";
        md.to_file(path).expect("保存文件失败");

        // 检查文件是否存在
        assert!(Path::new(path).exists(), "生成的文件不存在");

        // 检查内容是否包含关键字段
        let content = fs::read_to_string(path).expect("读取文件失败");
        assert!(content.contains("title: Test Title"));
        assert!(content.contains("author: Test Author"));
        assert!(content.contains("- rust"));
        assert!(content.contains("This is a test markdown content."));

        println!("已生成测试文件: {}", path);

        // ---------- 测试生成临时文件 ----------
        let tmp_file = md.to_tempfile().expect("生成临时文件失败");

        // 检查临时文件路径存在
        let tmp_path = tmp_file.path();
        assert!(tmp_path.exists(), "临时文件不存在");

        let tmp_content = fs::read_to_string(tmp_path).expect("读取临时文件失败");
        assert!(tmp_content.contains("title: Test Title"));
        assert!(tmp_content.contains("author: Test Author"));
        assert!(tmp_content.contains("This is a test markdown content."));

        println!("临时文件路径: {:?}", tmp_path);
    }

    #[test]
    fn test_markdown_to_hexo() {
        let md = Markdown {
            author: "Alice".to_string(),
            title: "My Post".to_string(),
            tags: vec!["rust".to_string(), "hexo".to_string()],
            content: "Hello, world!".to_string(),
        };

        let hexo_str = md.to_hexo();

        // 检查 front matter 基础结构
        assert!(hexo_str.contains("---"));
        assert!(hexo_str.contains("title: My Post"));
        assert!(hexo_str.contains("author: Alice"));
        assert!(hexo_str.contains("tags:"));
        assert!(hexo_str.contains("- rust"));
        assert!(hexo_str.contains("- hexo"));
        assert!(hexo_str.contains("Hello, world!"));

        // 检查 date 格式是否是 yyyy-MM-dd HH:mm:ss
        let date_line = hexo_str
            .lines()
            .find(|l| l.starts_with("date: "))
            .expect("date line should exist");

        let date_str = date_line.trim_start_matches("date: ").trim();
        assert!(
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").is_ok(),
            "date format should be valid, got {}",
            date_str
        );
    }
}