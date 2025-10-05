use crate::config::AppConfig;
use crate::handler::submit::SubmissionRequest;
use crate::utils::file::{Markdown, ToHexo};
use crate::utils::picture::Base64Image;
use git2::{Cred, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Signature};
use octocrab::Octocrab;
use secrecy::ExposeSecret;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

pub struct Submission {
    pub author: String,
    pub email: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
    pub cover: Base64Image,
    pub images: Vec<Base64Image>,
    pub branch:String,
}

impl Submission {
    pub fn to_markdown(&self) -> Markdown {
        Markdown{
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
            self.images.iter()
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
}

impl ToHexo for Submission {
    fn to_hexo(&self) -> String {
        self.to_markdown().to_hexo()
    }
}

impl Submission{
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
    pub fn from_request(submission_request:SubmissionRequest)->Self{
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
    pub fn push_branch(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = AppConfig::global();
        let repo_url = config.github.repo_path.clone();

        // 1 临时目录
        let tmp_dir = tempdir()?;
        let repo_path = tmp_dir.path();

        // 2 克隆仓库
        let repo = Repository::clone(&repo_url, repo_path)?;

        // 3 创建唯一分支
        let head_ref = repo.head()?.target().expect("HEAD should point to a commit");
        let branch = repo.branch(&self.branch, &repo.find_commit(head_ref)?, false)?;
        let branch_ref = branch.get().name().unwrap();
        let obj = repo.revparse_single(branch_ref)?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(branch_ref)?;

        // 4 保存 Markdown
        let md_path = repo_path.join(format!("source/_posts/{}.md", self.title));
        fs::write(&md_path, self.to_hexo())?;

        // 保存 cover
        let cover_path = repo_path.join(format!("source/_posts/{}/cover.webp", self.title));
        self.cover.save(&cover_path)?;

        // 保存其他图片
        for (idx, img) in self.images.iter().enumerate() {
            let img_path = repo_path.join(format!("source/photos/{}/{}.webp", self.title, idx+1));
            img.save(&img_path)?;
        }

        // 5 git add + commit
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;
        let sig = Signature::now(&self.author, &self.email)?;
        let parent_commit = repo.find_commit(head_ref)?;
        repo.commit(Some("HEAD"), &sig, &sig, "Add new submission", &tree, &[&parent_commit])?;

        // 6 push 分支 - 使用 PAT 进行认证
        let pat = config.github.personal_access_token.expose_secret().clone();
        let pat_clone = pat.clone();
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, _user, _cred| {
            // 使用 PAT 作为密码，用户名为 "git"
            Cred::userpass_plaintext("git", &pat_clone)
        });
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);
        let mut remote = repo.find_remote("origin")?;
        remote.push(&[&format!("refs/heads/{}", self.branch)], Some(&mut push_options))?;

        println!("push branch '{}'", self.branch);
        Ok(())
    }

    pub async fn pull_request(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = AppConfig::global();
        let pat = config.github.personal_access_token.expose_secret().clone();

        let repo_url_clone =AppConfig::global().github.repo_path.clone();
        let parts: Vec<String> = repo_url_clone
            .trim_end_matches(".git").rsplitn(3, '/')
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
            .map_err(|e| format!("Failed to build Octocrab: {}", e))?;

        let pr = octocrab
            .pulls(owner_name, repo_name)
            .create(pr_title, self.branch.clone(), "main")
            .body(pr_body)
            .send()
            .await
            .map_err(|e| format!("Failed to create PR: {}", e))?;

        println!("pull request branch '{}'", self.branch);
        Ok(())
    }
}

#[cfg(test)]
mod github_tests {
    use super::*;
    use git2::{Cred, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Signature};
    use octocrab::params::pulls::State;
    use octocrab::Octocrab;
    use std::fs;
    use tempfile::tempdir;
    use uuid::Uuid;

    // 1x1 像素的 WEBP 图像（白色）
    const TEST_WEBP_BASE64: &str = "UklGRh4AAABXRUJQVlA4TBEAAAAvAAAAAAfQ//73v/+BiOh/AAA=";

    // 只有在有complete参数时才进行该段测试
    #[tokio::test]
    async fn test_submission_pull_request_sandbox() -> Result<(), Box<dyn std::error::Error>> {
        if !std::env::args().any(|arg| arg == "--complete") {
            eprintln!("Skipping GitHub test because --complete was not provided");
            return Ok(());
        }

        // 构造测试 Submission
        let submission = Submission {
            author: "Test Author".to_string(),
            email: "test@example.com".to_string(),
            title: "Test PR".to_string(),
            tags: vec!["rust".to_string()],
            content: "Testing PR creation.".to_string(),
            cover: Base64Image::new(TEST_WEBP_BASE64.to_string(), "cover.webp".to_string()),
            images: vec![],
            branch:"".to_string(),
        };

        let config = AppConfig::global();
        let repo_url = config.github.repo_path.clone();
        let pat = config.github.personal_access_token.expose_secret().clone(); // 使用 PAT
        let owner_repo: Vec<&str> = repo_url.trim_end_matches(".git").rsplitn(3, '/').collect();
        let repo_name = owner_repo[0];
        let owner_name = owner_repo[1];

        // 1. 临时目录
        let tmp_dir = tempdir()?;
        let repo_path = tmp_dir.path();

        // 2. 克隆仓库
        let pat_clone = pat.clone();
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, _user, _cred| {
            // 使用 PAT 作为密码
            Cred::userpass_plaintext("git", &pat_clone)
        });

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let repo = Repository::clone(repo_url.as_str(), repo_path)?;

        // 3. 创建唯一分支
        let branch_name = format!("test-contrib-{}", Uuid::new_v4());
        let head_ref = repo.head()?.target().unwrap();
        let commit = repo.find_commit(head_ref)?;
        let branch = repo.branch(&branch_name, &commit, false)?;
        let branch_ref = branch.get().name().unwrap();
        let obj = repo.revparse_single(branch_ref)?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(branch_ref)?;

        // 4. 保存 Markdown
        let md_path = repo_path.join(format!("source/_posts/{}.md", submission.title));
        fs::write(&md_path, submission.to_hexo())?;

        // 保存 cover
        let cover_path = repo_path.join(format!("source/_posts/{}/cover.webp", submission.title));
        submission.cover.save(&cover_path)?;

        // 5. git add + commit
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;
        let sig = Signature::now(&submission.author, &submission.email)?;
        repo.commit(Some("HEAD"), &sig, &sig, "Test submission commit", &tree, &[&commit])?;

        // 6. push 临时分支 - 使用 PAT
        let mut callbacks = RemoteCallbacks::new();
        let pat_clone = pat.clone();
        callbacks.credentials(move |_url, _user, _cred| {
            Cred::userpass_plaintext("git", &pat_clone)
        });
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);
        let mut remote = repo.find_remote("origin")?;
        remote.push(&[&format!("refs/heads/{}", branch_name)], Some(&mut push_options))?;

        // 7. 创建 PR - 使用 PAT
        let octocrab = Octocrab::builder().personal_token(pat.to_string()).build()?;
        let pr_title = format!("{}-{}", submission.title, submission.author);
        let tags_str = if submission.tags.is_empty() {
            "None".to_string()
        } else {
            submission.tags.join(", ")
        };
        let pr_body = format!(
            "Automated submission from contribution form.\n\n\
            **Title:** {}\n\
            **Author:** {}\n\
            **Tags:** {}\n\
            **Images:** {} (including cover)\n",
            submission.title,
            submission.author,
            tags_str,
            1 + submission.images.len()
        );

        let pr = octocrab
            .pulls(owner_name, repo_name)
            .create(&pr_title, &branch_name, "main")
            .body(pr_body)
            .send()
            .await?;

        println!("PR created at {}", pr.html_url.unwrap());

        // 8. 验证 PR 成功 → 关闭
        octocrab.pulls(owner_name, repo_name)
            .update(pr.number)
            .state(State::Closed)
            .send()
            .await?;

        println!("PR closed.");

        // 9. 删除远程分支
        let mut remote = repo.find_remote("origin")?;
        remote.push(&[&format!(":refs/heads/{}", branch_name)], Some(&mut push_options))?;
        println!("Remote branch deleted.");

        Ok(())
    }
}