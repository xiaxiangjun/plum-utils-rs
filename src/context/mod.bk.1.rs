use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub struct Context {
    internal: Arc<ContextInternal>,
    is_clone: bool,
}

struct ContextInternal {
    list: Mutex<Option<HashMap<i64, Waker>>>,
    index: AtomicI64,
}

struct ContextWaiter {
    internal: Arc<ContextInternal>,
    current: Option<i64>,
}

impl Context {
    // 创建一个新对像
    pub fn new() -> Self {
        Context {
            internal: Arc::new(ContextInternal {
                list: Mutex::new(Some(HashMap::new())),
                index: AtomicI64::new(5),
            }),
            is_clone: false,
        }
    }

    // 从已经存的context创建一个context, 析构时不执行清理
    pub fn with_context(ctx: &Context) -> Self {
        Context {
            is_clone: true,
            internal: ctx.internal.clone(),
        }
    }

    // 执行wait
    pub async fn done(&self) {
        ContextWaiter {
            internal: self.internal.clone(),
            current: None,
        }
            .await;
    }

    // 完成所有任务
    pub fn close(&mut self) {
        let mut list = self.internal.list.lock().unwrap();
        if let Some(list) = list.take() {
            // 将所有的waker唤醒
            for (_, waker) in list.iter() {
                waker.clone().wake();
            }
        }
    }
}

// drop时清理
impl Drop for ContextWaiter {
    fn drop(&mut self) {
        if let Some(index) = self.current {
            let mut list = self.internal.list.lock().unwrap();
            if let Some(list) = &mut *list {
                let _ = list.remove(&index);
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // 如果是克隆
        if self.is_clone {
            return;
        }

        self.close();
    }
}

impl Future for ContextWaiter {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let internal = self.internal.clone();
        let mut list = internal.list.lock().unwrap();
        // 判断是否已经退出
        if let Some(list) = &mut *list {
            if None == self.current {
                // 执行相应的等待操作
                let index = self.internal.index.fetch_add(1, Ordering::SeqCst);
                self.current = Some(index);
                // 将需要唤醒例程加入队列
                list.insert(index, cx.waker().clone());
            }

            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
