use std::collections::LinkedList;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub struct Context {
    internal: Arc<ContextInternal>,
}

struct ContextInternal {
    list: Arc<Mutex<LinkedList<Arc<Mutex<(bool, Option<Waker>)>>>>>,
    is_ok: Arc<AtomicBool>,
}

pub struct ContextWaiter {
    internal: Arc<ContextWaiterInternal>,
}

struct ContextWaiterInternal {
    waker: Arc<Mutex<(bool, Option<Waker>)>>,
    is_ok: Arc<AtomicBool>,
}

impl Context {
    // 创建一个新对像
    pub fn new() -> Self {
        Context {
            internal: Arc::new(ContextInternal {
                list: Arc::new(Mutex::new(LinkedList::new())),
                is_ok: Arc::new(AtomicBool::new(true)),
            }),
        }
    }

    // 执行wait
    pub fn waiter(&self) -> ContextWaiter {
        let waker = Arc::new(Mutex::new((true, None)));
        // 存府waker
        self.internal
            .list
            .lock()
            .as_mut()
            .unwrap()
            .push_back(waker.clone());

        ContextWaiter {
            internal: Arc::new(ContextWaiterInternal {
                waker,
                is_ok: self.internal.is_ok.clone(),
            }),
        }
    }

    // 完成所有任务
    pub fn close(&self) {
        self.internal.close();
    }
}

impl ContextInternal {
    fn close(&self) {
        // 存储关闭事件
        self.is_ok.store(false, Ordering::SeqCst);

        // 清理所有waker
        let list = self.list.lock().unwrap();
        // 将所有的waker唤醒
        for item in list.iter() {
            let mut waker = item.lock().unwrap();
            waker.0 = false;
            if let Some(w) = waker.1.take() {
                w.wake();
            }
        }
    }
}

// drop时清理
impl Drop for ContextWaiterInternal {
    fn drop(&mut self) {
        let mut waker = self.waker.lock().unwrap();
        waker.0 = false;
        waker.1.take();
    }
}

impl Drop for ContextInternal {
    fn drop(&mut self) {
        self.close();
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            internal: self.internal.clone(),
        }
    }
}

impl Clone for ContextWaiter {
    fn clone(&self) -> Self {
        ContextWaiter {
            internal: self.internal.clone(),
        }
    }
}

impl Future for ContextWaiter {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let internal = self.internal.clone();
        // 判断对像是否有效
        if internal.is_ok.load(Ordering::SeqCst) {
            let mut waker = internal.waker.lock().unwrap();
            if waker.0 == false {
                return Poll::Ready(());
            }

            if let Some(w) = waker.1.as_ref() {
                // 存在变量，如果不相同则为替换
                if cx.waker().will_wake(w) {
                    return Poll::Pending;
                }
            }

            waker.1.replace(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
