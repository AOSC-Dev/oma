//! 追踪由 [`JoinSet`](tokio::task::JoinSet) 派生的子任务。
//!
//! 取消（`abort`）只会让运行时的下一个调度点丢弃子任务 future，而丢弃
//! 一个 `JoinSet` 也只是给其中的子任务发取消信号、不等待它们退出。对于
//! 退出前会写列表目录的下载任务，取消方在释放目录锁之前必须再等到所有
//! 子任务真正析构，否则已经跑了一半的子任务仍可能在锁释放后落盘、覆盖
//! 新刷新写下的元数据。
//!
//! 用法：派生子任务前 [`TaskTracker::guard`] 登记，并把 guard 移入该任务
//! 的 future（`async move { let _guard = guard; ... }`）——guard 随 future
//! 一起析构时自动注销。取消方调用 [`TaskTracker::wait`] 阻塞到计数归零。

use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug, Default)]
struct Inner {
    /// 已登记、尚未注销的子任务数。
    alive: Mutex<usize>,
    /// 计数归零时唤醒等待方。
    drained: Condvar,
}

/// 子任务存活追踪器，见[模块文档](self)。
#[derive(Clone, Debug, Default)]
pub struct TaskTracker {
    inner: Arc<Inner>,
}

impl TaskTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一个子任务。返回的 guard 应移入该子任务的 future，随 future
    /// 一起被丢弃——无论它是正常结束还是被 `abort` 取消。
    #[must_use]
    pub fn guard(&self) -> TaskGuard {
        *self.inner.alive.lock().unwrap() += 1;

        TaskGuard {
            inner: self.inner.clone(),
        }
    }

    /// 阻塞等待所有已登记的子任务注销（对应的 future 已析构）。
    pub fn wait(&self) {
        let mut alive = self.inner.alive.lock().unwrap();

        while *alive != 0 {
            alive = self.inner.drained.wait(alive).unwrap();
        }
    }
}

/// [`TaskTracker`] 的登记凭据，析构时注销对应的子任务。
#[derive(Debug)]
pub struct TaskGuard {
    inner: Arc<Inner>,
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        let mut alive = self.inner.alive.lock().unwrap();
        *alive -= 1;

        if *alive == 0 {
            self.inner.drained.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn wait_without_guard_returns() {
        TaskTracker::new().wait();
    }

    #[test]
    fn wait_blocks_until_all_guards_drop() {
        let tracker = TaskTracker::new();
        let guard1 = tracker.guard();
        let guard2 = tracker.guard();

        let tracker_in_thread = tracker.clone();
        let (tx, rx) = flume::bounded(1);
        let thread = std::thread::spawn(move || {
            tracker_in_thread.wait();
            let _ = tx.send(());
        });

        // 还有子任务存活：wait 不应返回。
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_err());

        drop(guard1);
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_err());

        drop(guard2);
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
        thread.join().unwrap();
    }

    #[test]
    fn dropping_future_releases_guard() {
        let tracker = TaskTracker::new();

        let tracker_in_thread = tracker.clone();
        let (tx, rx) = flume::bounded(1);
        let thread = std::thread::spawn(move || {
            tracker_in_thread.wait();
            let _ = tx.send(());
        });

        // guard 被移入 future 后一起丢弃（从未被轮询也适用，类似任务在
        // 首次调度前就被取消丢弃）。
        let guard = tracker.guard();
        let fut = async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        };
        drop(fut);

        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
        thread.join().unwrap();
    }
}
