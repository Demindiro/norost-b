use super::io;
use core::{
	fmt,
	marker::PhantomData,
	mem::{self, MaybeUninit},
	ptr::NonNull,
};

pub use norostb_kernel::{
	io::{DoIo, Job},
	object::NewObject,
	Handle,
};

#[derive(Debug)]
pub struct Object(Handle);

impl Object {
	/// Create a new local object.
	#[inline(always)]
	pub fn new(args: NewObject) -> io::Result<Self> {
		io::new_object(args).map(Self)
	}

	#[inline(always)]
	pub fn open(&self, path: &[u8]) -> io::Result<Self> {
		io::open(self.0, path).map(Self)
	}

	#[inline(always)]
	pub fn create(&self, path: &[u8]) -> io::Result<Self> {
		io::create(self.0, path).map(Self)
	}

	#[inline(always)]
	pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
		io::read(self.0, buf)
	}

	#[inline]
	pub fn read_uninit<'a>(
		&self,
		buf: &'a mut [MaybeUninit<u8>],
	) -> io::Result<(&'a mut [u8], &'a mut [MaybeUninit<u8>])> {
		io::read_uninit(self.0, buf).map(|l| {
			let (i, u) = buf.split_at_mut(l);
			// SAFETY: all bytes in i are initialized
			(unsafe { MaybeUninit::slice_assume_init_mut(i) }, u)
		})
	}

	#[inline(always)]
	pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
		io::peek(self.0, buf)
	}

	#[inline]
	pub fn peek_uninit<'a>(
		&self,
		buf: &'a mut [MaybeUninit<u8>],
	) -> io::Result<(&'a mut [u8], &'a mut [MaybeUninit<u8>])> {
		io::peek_uninit(self.0, buf).map(|l| {
			let (i, u) = buf.split_at_mut(l);
			// SAFETY: all bytes in i are initialized
			(unsafe { MaybeUninit::slice_assume_init_mut(i) }, u)
		})
	}

	#[inline]
	pub fn write(&self, data: &[u8]) -> io::Result<usize> {
		io::write(self.0, data)
	}

	#[inline]
	pub fn seek(&self, pos: io::SeekFrom) -> io::Result<u64> {
		io::seek(self.0, pos)
	}

	#[inline]
	pub fn share(&self, share: &Object) -> io::Result<u64> {
		io::share(self.0, share.0)
	}

	#[inline]
	pub fn poll(&self) -> io::Result<u64> {
		io::poll(self.0)
	}

	#[inline]
	pub fn map_object(
		&self,
		base: Option<NonNull<u8>>,
		offset: u64,
		length: usize,
	) -> io::Result<NonNull<u8>> {
		io::map_object(self.0, base, offset, length)
	}

	#[inline]
	pub const fn as_raw(&self) -> Handle {
		self.0
	}

	#[inline]
	pub const fn into_raw(self) -> Handle {
		let h = self.0;
		mem::forget(self);
		h
	}

	#[inline(always)]
	pub fn as_ref_object(&self) -> RefObject<'_> {
		RefObject {
			handle: self.0,
			_marker: PhantomData,
		}
	}

	#[inline]
	pub const fn from_raw(handle: Handle) -> Self {
		Self(handle)
	}

	/// Convienence method for use with `write!()` et al.
	pub fn write_fmt(&self, args: fmt::Arguments<'_>) -> io::Result<()> {
		struct Fmt {
			obj: Handle,
			res: io::Result<()>,
		}
		impl fmt::Write for Fmt {
			fn write_str(&mut self, s: &str) -> fmt::Result {
				io::write(self.obj, s.as_bytes()).map(|_| ()).map_err(|e| {
					self.res = Err(e);
					fmt::Error
				})
			}
		}
		let mut f = Fmt {
			obj: self.0,
			res: Ok(()),
		};
		let _ = fmt::write(&mut f, args);
		f.res
	}
}

impl Drop for Object {
	/// Close the handle to this object.
	fn drop(&mut self) {
		io::close(self.0)
	}
}

/// An object by "reference" but with less indirection.
#[derive(Clone, Copy)]
pub struct RefObject<'a> {
	handle: Handle,
	_marker: PhantomData<&'a Object>,
}

impl<'a> RefObject<'a> {
	#[inline]
	pub const fn from_raw(handle: Handle) -> Self {
		Self {
			handle,
			_marker: PhantomData,
		}
	}

	#[inline]
	pub const fn as_raw(&self) -> Handle {
		self.handle
	}

	#[inline]
	pub const fn into_raw(self) -> Handle {
		self.handle
	}
}

impl<'a> core::ops::Deref for RefObject<'a> {
	type Target = Object;

	fn deref(&self) -> &Self::Target {
		// SAFETY: Object is a simple wrapper around the handle.
		unsafe { mem::transmute(&self.handle) }
	}
}
