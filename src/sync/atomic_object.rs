use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AtomicObject<T: Sized> {
    ptr: AtomicUsize,
    data: PhantomData<T>,
}

impl<T: Sized> AtomicObject<T> {
    // 构建一个默认的对像
    pub fn default() -> Self {
        AtomicObject {
            ptr: AtomicUsize::new(0),
            data: PhantomData::default(),
        }
    }

    // 存储对像
    pub fn store(&self, t: T) {
        let p = Box::new(t);
        let old = self.ptr.swap(Box::into_raw(p) as usize, Ordering::SeqCst);
        if 0 == old {
            return;
        }

        // 释放旧的数据
        unsafe {
            let _ = Box::from_raw(old as *mut T);
        }
    }

    // 加载对像
    pub fn load(&self) -> Option<Box<T>> {
        let t = self.ptr.swap(0, Ordering::SeqCst);
        if 0 == t {
            return None;
        }

        Some(unsafe { Box::from_raw(t as *mut T) })
    }
}

impl<T: Sized> Drop for AtomicObject<T> {
    fn drop(&mut self) {
        let _ = self.load();
    }
}
