use crate::config::AppConfig;
use crate::handler::submit::SubmissionRequest;
use crate::utils::markdown::{Markdown, ToHexo};
use crate::utils::picture::Base64Image;
use anyhow::{Context, Result, anyhow};
use octocrab::Octocrab;
use octocrab::models::repos::Object;
use octocrab::params::repos::Reference;
use secrecy::ExposeSecret;
use urlencoding::encode;
use uuid::Uuid;

pub struct Submission {
    pub author: String,
    pub email: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
    pub cover: Base64Image,
    pub images: Vec<Base64Image>,
    pub branch: String,
}

impl Submission {
    pub fn to_markdown(&self) -> Markdown {
        Markdown {
            author: self.author.clone(),
            title: self.title.clone(),
            tags: self.tags.clone(),
            content: self.content.clone(),
        }
    }

    pub fn to_info(&self) -> String {
        let additional_images = if self.images.is_empty() {
            "无".to_string()
        } else {
            self.images
                .iter()
                .map(|img| img.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        };

        format!(
            "新投稿提醒:\n\
            作者: {}\n\
            邮箱: {}\n\
            标题: {}\n\
            标签: {}\n\
            内容长度: {} 字符\n\
            封面图片: {}\n\
            附加图片: {}\n\
            分支名: {}\
            ",
            self.author,
            self.email,
            self.title,
            self.tags.join(", "),
            self.content.chars().count(),
            self.cover.name,
            additional_images,
            self.branch
        )
    }
    pub fn to_title(&self) -> String {
        format!("{}-{}-{}", self.author, self.email, self.title)
    }

    pub fn to_contributor(&self, pr_url: &str) -> String {
        format!(
            r#"感谢您的投稿！

投稿详情：
- 文章标题：《{}》
- 作者：{}
- 标签：{}
- 投稿邮箱：{}

您的投稿已成功提交！我们已创建 GitHub Pull Request 进行审核处理。

🔗 查看处理进度：{}
（如链接无法点击，请复制到浏览器打开）

审核流程：
1. 管理员将会审核您的投稿内容
2. 审核通过后，您的文章将会被发布
3. 如有需要修改的地方，我们会通过邮件与您沟通

预计审核时间：1-3个工作日
如有任何问题，请回复此邮件与我们联系。

再次感谢您对科幻文学的支持！"#,
            self.title,
            self.author,
            self.tags.join("、"),
            self.email,
            pr_url
        )
    }
}

impl ToHexo for Submission {
    fn to_hexo(&self) -> String {
        self.to_markdown().to_hexo()
    }
}

impl Submission {
    pub fn new(
        author: String,
        email: String,
        title: String,
        tags: Vec<String>,
        content: String,
        cover: Base64Image,
        images: Vec<Base64Image>,
    ) -> Self {
        let branch = format!("contrib-{}", Uuid::new_v4());
        Self {
            author,
            email,
            title,
            tags,
            content,
            cover,
            images,
            branch,
        }
    }
    pub fn from_request(submission_request: SubmissionRequest) -> Self {
        Submission::new(
            submission_request.author,
            submission_request.email,
            submission_request.title,
            submission_request.tags,
            submission_request.content,
            submission_request.cover,
            submission_request.images,
        )
    }
    pub async fn push_branch(&self) -> Result<()> {
        let config = AppConfig::global();
        let repo_url = config.github.repo_path.clone();
        let pat = config.github.personal_access_token.expose_secret().clone();

        // 提取 owner/repo
        let parts: Vec<String> = repo_url
            .trim_end_matches(".git")
            .rsplitn(3, '/')
            .map(|p| p.to_string())
            .collect();
        let repo_name = parts[0].clone();
        let owner_name = parts[1].clone();

        let octocrab = Octocrab::builder()
            .personal_token(pat.clone())
            .build()
            .context("构建 Octocrab 客户端失败")?;

        // 1 获取 main 分支最新 SHA
        let main_ref = octocrab
            .repos(owner_name.clone(), repo_name.clone())
            .get_ref(&Reference::Branch("main".to_string()))
            .await
            .context("获取 main 分支引用失败")?;

        let main_sha = match main_ref.object {
            Object::Commit { sha, .. } => sha,
            _ => return Err(anyhow!("heads/main 未指向 Commit 对象")),
        };

        // 2 创建唯一分支（指向 main）
        octocrab
            .repos(owner_name.clone(), repo_name.clone())
            .create_ref(&Reference::Branch(self.branch.clone()), main_sha)
            .await
            .context("创建分支失败")?;

        // 工具闭包：对 URL 的每个路径段做百分号编码
        let encode_path = |p: &str| {
            p.split('/')
                .map(|seg| encode(seg).into_owned())
                .collect::<Vec<_>>()
                .join("/")
        };

        // 3 提交 Markdown
        let md_path_encoded = encode_path(&format!("source/_posts/{}.md", self.title));
        let md_bytes = self.to_hexo().into_bytes();
        octocrab
            .repos(owner_name.clone(), repo_name.clone())
            .create_file(md_path_encoded, "Add new submission: markdown", md_bytes)
            .branch(&self.branch)
            .send()
            .await
            .context("提交 Markdown 文件失败")?;

        // 4 保存 cover
        let cover_path_encoded = encode_path(&format!("source/_posts/{}/cover.webp", self.title));
        let cover_bytes = self.cover.to_bytes().context("封面图片编码失败")?;

        octocrab
            .repos(owner_name.clone(), repo_name.clone())
            .create_file(cover_path_encoded, "Add new submission: cover", cover_bytes)
            .branch(&self.branch)
            .send()
            .await
            .context("提交封面文件失败")?;

        // 5 保存其他图片
        for (idx, img) in self.images.iter().enumerate() {
            let img_path_encoded =
                encode_path(&format!("source/photos/{}/{}.webp", self.title, idx + 1));
            let img_bytes = img.to_bytes().context("附加图片编码失败")?;
            octocrab
                .repos(owner_name.clone(), repo_name.clone())
                .create_file(img_path_encoded, "Add new submission: image", img_bytes)
                .branch(&self.branch)
                .send()
                .await
                .with_context(|| format!("提交第 {} 张图片失败", idx + 1))?;
        }

        // 6 完成
        println!("push branch '{}' success", self.branch);
        Ok(())
    }

    pub async fn pull_request(&self) -> Result<String> {
        let config = AppConfig::global();
        let pat = config.github.personal_access_token.expose_secret().clone();

        let repo_url_clone = AppConfig::global().github.repo_path.clone();
        let parts: Vec<String> = repo_url_clone
            .trim_end_matches(".git")
            .rsplitn(3, '/')
            .map(|p| p.to_string())
            .collect();
        let repo_name = parts[0].clone();
        let owner_name = parts[1].clone();

        let pr_title = format!("{}-{}", self.title, self.author);
        // PR body 包含基本信息
        let tags_str = if self.tags.is_empty() {
            "None".to_string()
        } else {
            self.tags.join(", ")
        };
        let pr_body = format!(
            "Automated submission from contribution form.\n\n\
            **Title:** {}\n\
            **Author:** {}\n\
            **Email:** {}\n\
            **Tags:** {}\n\
            **Images:** {} (including cover)\n",
            self.title,
            self.author,
            self.email,
            tags_str,
            1 + self.images.len(),
        );

        let octocrab = Octocrab::builder()
            .personal_token(pat.to_string())
            .build()
            .context("构建 Octocrab 客户端失败")?;

        let pr = octocrab
            .pulls(owner_name.clone(), repo_name.clone())
            .create(pr_title, self.branch.clone(), "main")
            .body(pr_body)
            .send()
            .await
            .context("创建 Pull Request 失败")?;

        let url = pr
            .html_url
            .map(|url| url.to_string())
            .unwrap_or_else(|| {
                format!("https://github.com/{}/{}/pull/{}", &owner_name, &repo_name, pr.number)
            });

        println!("pull request branch '{}'", self.branch);
        Ok(url)
    }
}
