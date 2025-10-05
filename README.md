
---

# QidianMini 后端服务 ⚡

> **“为奇点科普科幻协会提供稳定的投稿与认证后端服务。”**
> —— QidianMini 负责处理用户投稿、邮箱验证码、Github OAuth 登录以及图片上传。

---

## 🚀 项目概览

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.78+-orange.svg)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.8.5-brightgreen.svg)](https://github.com/tokio-rs/axum)

* **语言/框架：** Rust + [Axum](https://github.com/tokio-rs/axum)
* **功能：**
    * 投稿接口 `/auth/send`、`/submit`
    * Github OAuth 授权
    * SMTP 邮件验证码发送
    * 图片上传与处理
* **部署方式：** systemd + Nginx 反向代理 + HTTPS
* **端口：** 默认 4502

---

## 📂 目录结构

```text
src/
├── config.rs           # 配置读取逻辑
├── handler/            # 路由处理逻辑
│   ├── auth.rs
│   ├── submit.rs
│   └── mod.rs
├── middleware/         # 中间件
│   ├── cors.rs         # CORS 配置
│   ├── mem_map.rs
│   └── mod.rs
├── response.rs         # API 响应封装
├── routes/             # 路由定义
│   ├── auth.rs
│   ├── health.rs
│   ├── submit.rs
│   └── mod.rs
├── utils/              # 工具库
│   ├── email.rs
│   ├── file.rs
│   ├── github.rs
│   ├── picture.rs
│   └── mod.rs
└── main.rs             # 程序入口
```

配置文件：

* `config.toml`

```toml
[app]
port = 4502

[github]
redirect_uri = "https://contribute.qidian.space"
repo_path = "https://github.com/qidiankepukehuan/qidiankepukehuan"

[smtp]
username = "<SMTP邮箱用户名>"
host = "smtp.163.com"

[admin]
emails = [
    "管理员邮箱1",
    "管理员邮箱2",
    "管理员邮箱3"
]
```

* `.env`

```env
# Github OAuth
QIDIAN_MINI_GITHUB_CLIENT_ID=<你的Github客户端ID>
QIDIAN_MINI_GITHUB_CLIENT_SECRET=<你的Github客户端密钥>
QIDIAN_MINI_GITHUB_PAT=<Github个人访问令牌>

# SMTP 邮箱密码
QIDIAN_MINI_SMTP_PASSWORD=<SMTP邮箱授权码或密码>
```

---

## ⚙️ 部署指南

### 1️⃣ 编译程序

```bash
git clone  QidianMini
cd QidianMini
cargo build --release
```

编译执行后可执行文件在：

```
target/release/QidianMini
```

---

### 2️⃣ 配置 systemd 服务

创建 `/etc/systemd/system/qidianmini.service`：

```ini
[Unit]
Description=QidianMini Backend Service
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/QidianMini
Restart=on-failure
EnvironmentFile=/etc/qidianmini/.env
WorkingDirectory=/usr/local/bin
User=root

[Install]
WantedBy=multi-user.target
```

启用并启动服务：

```bash
sudo systemctl enable qidianmini
sudo systemctl start qidianmini
sudo systemctl status qidianmini
```

---

### 3️⃣ Nginx 反向代理 + HTTPS

配置示例：

```nginx
server {
    listen 80;
    server_name contribute.qidian.space;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name contribute.qidian.space;

    ssl_certificate     /etc/letsencrypt/live/contribute.qidian.space/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/contribute.qidian.space/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:4502;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

---

### 4️⃣ 更新程序流程

```bash
# 停止服务
sudo systemctl stop qidianmini

# 替换二进制
sudo cp target/release/QidianMini /usr/local/bin/QidianMini
sudo chmod +x /usr/local/bin/QidianMini

# 启动服务
sudo systemctl start qidianmini

# 查看日志
sudo journalctl -u qidianmini -f
```

---

## 👀 联系方式

* 协会邮箱：[tsblydyzbjb@qidian.space](mailto:tsblydyzbjb@qidian.space)
* Github 仓库：`https://github.com/qidiankepukehuan/qidian_mini`

---

## 📜 许可证

MIT License


---