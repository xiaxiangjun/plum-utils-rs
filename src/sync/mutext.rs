use std::cell::UnsafeCell;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

pub struct Mutex<T: Sized> {
    pub lock_index: AtomicUsize,      // 锁ID自增计数
    pub current_lock_id: AtomicUsize, // 当前获得锁的ID
    pub waiter_list: AtomicUsize,     // 等待列表 指向 *mut Box<MutexWaiter>
    data_ptr: UnsafeCell<T>,
}

unsafe impl<T: Sized + Send> Send for Mutex<T> {}
unsafe impl<T: Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    // 创建一个新对像
    pub fn new(t: T) -> Self {
        Mutex {
            data_ptr: UnsafeCell::new(t),
            lock_index: AtomicUsize::new(1000),
            current_lock_id: AtomicUsize::new(0),
            waiter_list: AtomicUsize::new(0),
        }
    }

    // 锁定对像
    pub async fn lock(&self) -> MutexGuard<'_, T> {
        // 生成一个新的ID
        let mut lock_id = 0;
        while 0 == lock_id {
            lock_id = self.lock_index.fetch_add(1, Ordering::SeqCst);
        }

        // 尝试获得控制权
        if Ok(0)
            != self
                .current_lock_id
                .compare_exchange(0, lock_id, Ordering::SeqCst, Ordering::SeqCst)
        {
            // 获法获得控制权时，需要等待别人释放控制权
            MutexAcquire {
                lock_id,
                lock: self,
            }
            .await;
        }

        // 返回结果
        MutexGuard {
            lock_id,
            lock: self,
        }
    }
}

#[cfg(Debug)]
impl<T: Sized> Drop for Mutex<T> {
    fn drop(&mut self) {
        if 0 != self.waiter_list.load(Ordering::SeqCst) {
            panic!("waiter is not empty");
        }
    }
}

pub struct MutexGuard<'a, T: Sized> {
    lock: &'a Mutex<T>,
    lock_id: usize,
}

impl<'a, T: Sized> MutexGuard<'a, T> {
    // 唤醒全部
    fn wake(&self, list: &Box<MutexWaiter>) {
        // 唤醒下一个节点
        if 0 != list.next {
            let next = unsafe { Box::from_raw(list.next as *mut MutexWaiter) };
            self.wake(&next);
        }

        let waker = unsafe { Box::from_raw(list.waker as *mut Waker) };
        waker.wake();
    }
}

impl<'a, T: Sized> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data_ptr.get() }
    }
}

impl<'a, T: Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data_ptr.get() }
    }
}

impl<'a, T: Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        // 释放锁
        if Ok(self.lock_id)
            != self.lock.current_lock_id.compare_exchange(
                self.lock_id,
                0,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
        {
            panic!(
                "MutexGuard error: current lock id {}, {}",
                self.lock_id,
                self.lock.current_lock_id.load(Ordering::SeqCst)
            );
        }

        // 取出链表
        let waiter_list = self.lock.waiter_list.swap(0, Ordering::SeqCst);
        if 0 != waiter_list {
            let list = unsafe { Box::from_raw(waiter_list as *mut MutexWaiter) };
            self.wake(&list);
        }
    }
}

struct MutexWaiter {
    next: usize,  // 指向 *mut MutexWaiter
    waker: usize, // 指向 *mut Waker
}

struct MutexAcquire<'a, T: Sized> {
    lock: &'a Mutex<T>,
    lock_id: usize,
}

impl<'a, T: Sized> Future for MutexAcquire<'a, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 尝试获取锁
        if Ok(0)
            == self.lock.current_lock_id.compare_exchange(
                0,
                self.lock_id,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
        {
            return Poll::Ready(());
        }

        // 分配空间，准备将waker存储到链表
        let waiter = Box::new(MutexWaiter {
            next: 0,
            waker: Box::into_raw(Box::new(cx.waker().clone())) as usize,
        });

        let waiter_ptr = Box::into_raw(waiter);

        loop {
            // 取出存储的head
            let head = self.lock.waiter_list.load(Ordering::SeqCst);
            let mut waiter = unsafe { Box::from_raw(waiter_ptr) };

            waiter.next = head;
            // 存储链表
            let ptr = Box::into_raw(waiter) as usize;

            // 再尝试获取一次锁
            if Ok(0)
                == self.lock.current_lock_id.compare_exchange(
                    0,
                    self.lock_id,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
            {
                let _ = unsafe { Box::from_raw(waiter_ptr) };
                return Poll::Ready(());
            }

            // 获取锁失败，再一次加入队列
            if Ok(head)
                == self.lock.waiter_list.compare_exchange(
                    head,
                    ptr,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
            {
                return Poll::Pending;
            }
        }
    }
}

#[cfg(test)]
struct MutexTest {}

#[cfg(test)]
impl MutexTest {
    async fn run(&self, flag: String) {
        println!("{} start: >>>>>>>>>>>>>>>>>>", &flag);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        println!("{} end: -------------------", &flag);
    }
}
#[tokio::test]
async fn test_mutex() {
    let global = std::sync::Arc::new(Mutex::new(MutexTest {}));

    let (send, mut recv) = tokio::sync::mpsc::channel::<()>(1);

    for i in 0..5 {
        let tx = send.clone();
        let g = global.clone();
        tokio::spawn(async move {
            let _tx = tx;
            // 尝试100次获得锁，并执行任务
            for k in 0..3 {
                tokio::time::sleep(std::time::Duration::from_millis(
                    std::time::SystemTime::now()
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64
                        % 100,
                ))
                .await;

                let flag = format!("[thread-{}] {}: ", i, k);
                g.lock().await.run(flag).await;
            }
        });
    }

    drop(send);
    recv.recv().await;
}
