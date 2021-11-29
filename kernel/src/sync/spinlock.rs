use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU8, Ordering};

/// A very basic spinlock implementation. Intended for short sections that are mostly uncontended.
pub struct SpinLock<T> {
	lock: AtomicU8,
	value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
	pub const fn new(value: T) -> Self {
		Self {
			lock: AtomicU8::new(0),
			value: UnsafeCell::new(value),
		}
	}

	#[track_caller]
	pub fn lock(&self) -> Guard<T> {
		loop {
			match self
				.lock
				.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
			{
				Ok(_) => return Guard { lock: self },
				Err(_) => {
					// Force unlock for now, even though that's unsafe
					self.lock.store(0, Ordering::Release);
					panic!("double lock on single core CPU!");
				}
			}
		}
	}
}

unsafe impl<T> Sync for SpinLock<T> {}

pub struct Guard<'a, T> {
	lock: &'a SpinLock<T>,
}

impl<T> Deref for Guard<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.lock.value.get() }
	}
}

impl<T> DerefMut for Guard<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.lock.value.get() }
	}
}

impl<T> Drop for Guard<'_, T> {
	fn drop(&mut self) {
		self.lock.lock.store(0, Ordering::Release);
	}
}
