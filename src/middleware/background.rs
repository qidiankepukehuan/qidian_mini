use once_cell::sync::Lazy;
use std::sync::{mpsc, Arc};
use std::thread;
use tracing::{error, info, warn};
use crate::utils::email::{Mailer, SmtpMailer};

/// 一条后台任务
type Job = Box<dyn FnOnce() + Send + 'static>;

/// 全局 Sender，用 std::sync::mpsc 即可
static JOB_TX: Lazy<mpsc::Sender<( &'static str, Job )>> = Lazy::new(|| {
    let (tx, rx) = mpsc::channel::<(&'static str, Job)>();

    // 启一个常驻 worker 线程，专门执行这些任务
    thread::spawn(move || {
        info!("TASK_POOL: worker thread started");

        for (name, job) in rx {
            info!("TASK_POOL[{name}]: started");

            // 防止某个任务 panic 把整个线程干崩
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)) {
                error!("TASK_POOL[{name}]: panicked: {:?}", e);
            } else {
                info!("TASK_POOL[{name}]: finished");
            }
        }

        info!("TASK_POOL: worker thread exiting (sender dropped)");
    });

    tx
});

/// 对外暴露：获取全局任务 sender
pub fn task_sender() -> &'static mpsc::Sender<(&'static str, Job)> {
    &JOB_TX
}

/// 提交一个后台任务到全局任务池
pub fn submit_background<F>(name: &'static str, f: F)
where
    F: FnOnce() + Send + 'static,
{
    // 如果队列满/发送失败，就打个日志，不影响主流程
    if let Err(e) = task_sender().send((name, Box::new(f))) {
        error!("TASK_POOL[{name}]: failed to enqueue job: {}", e);
    }
}

static MAIL: &str= "mail";

pub fn send_mail_background(
    mailer: Arc<SmtpMailer>,
    to: String,
    subject: String,
    body: String,
) {
    submit_background(MAIL,move || {
        if let Err(e) = mailer.send(&to, &subject, &body) {
            warn!("MAIL_BG[{MAIL}]: send mail to {} failed: {:#}", to, e);
        } else {
            info!(
                    "MAIL_BG[{MAIL}]: mail sent to {} (subject = {})",
                    to, subject
                );
        }
    });
}

