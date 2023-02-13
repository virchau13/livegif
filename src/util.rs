use std::{io::Write, ops::{Deref, DerefMut}, task::{Waker, Poll}, sync::Arc, cell::{RefCell, UnsafeCell}, pin::Pin};
use tokio::sync::RwLock as AsyncRwLock;
use tokio::runtime::Handle as TokioHandle;
use std::sync::RwLock;
use pin_project::pin_project;
use futures::{Stream, ready};

// Arc<tokio::sync::RwLock<T>> so I can implement Write on it
pub struct WritableArcAsyncRwLock<T: Write + Send + Sync>(Arc<AsyncRwLock<T>>, TokioHandle);

impl<T: Write + Send + Sync> WritableArcAsyncRwLock<T> {
    pub fn new(t: T, handle: TokioHandle) -> Self {
        Self(Arc::new(AsyncRwLock::new(t)), handle)
    }
    pub fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<T: Write + Send + Sync> Write for WritableArcAsyncRwLock<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.1.block_on(self.0.write()).write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.1.block_on(self.0.write()).flush()
    }
}

impl<T: Write + Send + Sync> Deref for WritableArcAsyncRwLock<T> {
    type Target = AsyncRwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Arc<tokio::sync::RwLock<T>> so I can implement Write on it
pub struct WritableArcRwLock<T: Write + Send + Sync>(Arc<RwLock<T>>);

impl<T: Write + Send + Sync> WritableArcRwLock<T> {
    pub fn new(t: T) -> Self {
        Self(Arc::new(RwLock::new(t)))
    }
    pub fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Write + Send + Sync> Write for WritableArcRwLock<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.write().unwrap().flush()
    }
}

impl<T: Write + Send + Sync> Deref for WritableArcRwLock<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// // Arc<RefCell<T>> so I can implement Write on it
// pub struct WritableArcRefCell<T: Write>(Arc<RefCell<T>>);
// 
// impl<T: Write> WritableArcRefCell<T> {
//     pub fn new(t: T) -> Self {
//         Self(Arc::new(RefCell::new(t)))
//     }
//     pub fn clone(&self) -> Self {
//         Self(self.0.clone())
//     }
// }
// 
// impl<T: Write> Write for WritableArcRefCell<T> {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         self.0.borrow_mut().write(buf)
//     }
// 
//     fn flush(&mut self) -> std::io::Result<()> {
//         self.0.borrow_mut().flush()
//     }
// }
// 
// impl<T: Write> Deref for WritableArcRefCell<T> {
//     type Target = RefCell<T>;
// 
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
// 
// const WRITABLE_STREAM_BASE_CAPACITY: usize = 1024;
// 
// pub struct WritableStream {
//     buf: AsyncRwLock<Vec<u8>>,
//     to_wake: Option<Waker>,
// }
// 
// impl WritableStream {
//     fn vector_with_base_capacity() -> Vec<u8> {
//         let mut v = Vec::new();
//         v.reserve(WRITABLE_STREAM_BASE_CAPACITY);
//         v
//     }
// }
// 
// impl WritableStream {
//     pub fn new() -> Self {
//         Self {
//             buf: AsyncRwLock::new(Self::vector_with_base_capacity()),
//             to_wake: None,
//         }
//     }
// }
// 
// impl Write for WritableStream {
//     fn write(&mut self, inp: &[u8]) -> std::io::Result<usize> {
//         self.buf.blocking_write().extend_from_slice(inp);
//         if inp.len() > 0 {
//             // we got data, ask the tasks to wake up
//             if let Some(waker) = self.to_wake.take() {
//                 waker.wake();
//             }
//         }
//         Ok(inp.len())
//     }
// 
//     fn flush(&mut self) -> std::io::Result<()> {
//         // noop
//         Ok(())
//     }
// }
// 
// impl Stream for WritableStream {
//     type Item = Vec<u8>;
// 
//     fn poll_next(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         // fuck it
//         let buf = self.buf.blocking_write();
//         match buf.len() {
//             0 => {
//                 // wait until more data comes
//                 self.to_wake = Some(cx.waker().clone());
//                 Poll::Pending
//             }
//             len => {
//                 // send all the current data
//                 let out = std::mem::replace(&mut *buf, Self::vector_with_base_capacity());
//                 Poll::Ready(Some(out))
//             }
//         }
//     }
// }
// 
// struct WritableUnsafeCell<T>(pub UnsafeCell<T>);
// 
// impl<T: Write> Write for WritableUnsafeCell<T> {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         unsafe { (*self.0.get()).write(buf) }
//     }
// 
//     fn flush(&mut self) -> std::io::Result<()> {
//         unsafe { (*self.0.get()).flush() }
//     }
// }
