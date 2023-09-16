use core::{
    cell::UnsafeCell,
    fmt::{Debug, Display},
    marker::PhantomData,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[cfg(feature = "std")]
use std::error::Error;

/// A buffer that can be filled once. Used for multi-threaded applications to create a "scrollback channel" where data can be accessed anywhere in the buffer while it's being filled.
pub struct SharedBuffer<'a, T: Sized> {
    buf: Box<[(AtomicBool, UnsafeCell<MaybeUninit<T>>)]>,
    filled_amt: AtomicUsize,
    _lt: PhantomData<&'a u8>,
}

/// Error containing the attempted pushed value. Returned only when the buffer is full.
pub struct PushError<T>(pub T);

impl<T> Debug for PushError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "PushError<{}> ( ??? )",
            std::any::type_name::<T>()
        ))
    }
}

impl<T> Display for PushError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "internal buffer is full, cannot push more values"
        ))
    }
}

#[cfg(feature = "std")]
impl<T> Error for PushError<T> {}

unsafe impl<'a, T> Send for SharedBuffer<'a, T> where T: Send {}
unsafe impl<'a, T> Sync for SharedBuffer<'a, T> where T: Sync {}

impl<'a, T> SharedBuffer<'a, T> {
    pub fn new(size: usize) -> Self {
        let mut buf = Vec::with_capacity(size);
        for _ in 0..size {
            buf.push((AtomicBool::new(false), MaybeUninit::uninit().into()))
        }
        Self {
            buf: buf.into(),
            filled_amt: AtomicUsize::new(0),
            _lt: PhantomData,
        }
    }

    pub fn buf_size(&'a self) -> usize {
        self.buf.len()
    }

    pub fn filled_size(&'a self) -> usize {
        self.filled_amt.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn push(&'a self, val: T) -> Result<(), PushError<T>> {
        // check if full
        let Ok(pos) = self
            .filled_amt
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                if x >= self.buf.len() {
                    None
                } else {
                    Some(x + 1)
                }
            })
        else {
            return Err(PushError(val));
        };

        // push value
        let cell = &self.buf[pos];
        // SAFETY: we currently are the only thread with mutable access to this cell,
        // and accessing it is blocked behind the AtomicBool via SharedBuffer::get
        unsafe {
            core::ptr::write(cell.1.get(), MaybeUninit::new(val));
        }
        cell.0.store(true, Ordering::SeqCst);
        return Ok(());
    }

    pub fn get(&'a self, pos: usize) -> Option<&'a T> {
        if pos >= self.filled_amt.load(Ordering::SeqCst) {
            None
        } else {
            let cell = &self.buf[pos];
            if !cell.0.load(Ordering::SeqCst) {
                // unfortunate luck: you hit the point between getting the cell, but the other
                // thread wasn't done writing it
                None
            } else {
                // SAFETY: this is initialized, indicated by AtomicBool. there isn't a thread
                // contesting this read because push cannot access the same cell.
                unsafe {
                    let uninit = &*cell.1.get();
                    Some(&*uninit.as_ptr())
                }
            }
        }
    }

    pub fn into_owned(self) -> Vec<T> {
        todo!()
    }
}

impl<'a, T> Drop for SharedBuffer<'a, T> {
    fn drop(&mut self) {
        for val in &self.buf[0..self.filled_amt.load(Ordering::SeqCst)] {
            debug_assert!(val.0.load(Ordering::SeqCst));
            let cell = val.1.get();
            // SAFETY: cell was initialized because it was behind self.filled_amt
            unsafe {
                (*cell).assume_init_drop();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nothing() {
        let buf = SharedBuffer::<f32>::new(0);
        assert!(buf.get(0).is_none());
        assert!(buf.push(0.0).is_err());
        assert!(buf.get(0).is_none());
    }
    
    #[test]
    fn just_one_item() {
        let buf = SharedBuffer::<f64>::new(1);
        assert!(buf.get(0).is_none());
        assert!(buf.push(0.0).is_ok());
        assert!(buf.get(0).copied().is_some_and(|x| x == 0.0));
    }

    #[test]
    fn something() {
        let buf = SharedBuffer::<i32>::new(5);
        for i in 0..5 {
            assert!(buf.get(i).is_none());
            assert!(buf.push(i as i32).is_ok());
            assert!(buf.get(i).copied().is_some_and(|x| x == i as i32));
        }
        assert!(buf.get(5).is_none());
        assert!(buf.push(5).is_err());
    }

    #[test]
    fn z_producer_consumer() {
        let buf = SharedBuffer::<u8>::new(10_000 * 10);
        std::thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    for x in 0..10_000 {
                        buf.push(if x % 2 == 0 { 5 } else { 6 }).unwrap();
                    }
                });
            }

            for range in [0..30_000, 2_000..50_000, 80_000..90_000, 0..100_000] {
                for x in range {
                    while buf.get(x).is_none() {}
                }
            }
        })
    }
}
