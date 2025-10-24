use chrono::Local;

#[derive(Clone)]
pub struct Markdown{
    pub author: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

pub trait ToHexo {
    fn to_hexo(&self) -> String;
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