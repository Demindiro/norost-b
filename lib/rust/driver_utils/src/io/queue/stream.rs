extern crate alloc;

use crate::Handle;
use alloc::vec::Vec;
use norostb_rt::{io, Error};

pub enum Job<'a> {
	Read {
		job_id: u32,
		handle: Handle,
		length: u64,
	},
	Peek {
		job_id: u32,
		handle: Handle,
		length: u64,
	},
	Write {
		job_id: u32,
		handle: Handle,
		data: &'a [u8],
	},
	Open {
		job_id: u32,
		handle: Handle,
		path: &'a [u8],
	},
	Create {
		job_id: u32,
		handle: Handle,
		path: &'a [u8],
	},
	Close {
		handle: Handle,
	},
	Seek {
		job_id: u32,
		handle: Handle,
		from: io::SeekFrom,
	},
}

macro_rules! with {
	(handle $fn:ident = $ty:ident, $f:ident) => {
		pub fn $fn(buf: &mut Vec<u8>, job_id: u32, handle: Handle) -> Result<(), ()> {
			buf.extend_from_slice(
				io::Job {
					ty: io::Job::$ty,
					job_id,
					handle,
					..Default::default()
				}
				.as_ref(),
			);
			Ok(())
		}
	};
	(buf $fn:ident = $ty:ident, $f:ident) => {
		pub fn $fn<F>(buf: &mut Vec<u8>, job_id: u32, $f: F) -> Result<(), ()>
		where
			F: FnOnce(&mut Vec<u8>) -> Result<(), ()>,
		{
			buf.extend_from_slice(
				io::Job {
					ty: io::Job::$ty,
					job_id,
					..Default::default()
				}
				.as_ref(),
			);
			$f(buf)
		}
	};
	(u64 $fn:ident = $ty:ident, $f:ident) => {
		pub fn $fn(buf: &mut Vec<u8>, job_id: u32, $f: u64) -> Result<(), ()> {
			buf.extend_from_slice(
				io::Job {
					ty: io::Job::$ty,
					job_id,
					..Default::default()
				}
				.as_ref(),
			);
			buf.extend_from_slice(&$f.to_ne_bytes());
			Ok(())
		}
	};
}

impl<'a> Job<'a> {
	pub fn deserialize(data: &'a [u8]) -> Option<Self> {
		let (job, data) = io::Job::deserialize(data)?;
		let (job_id, handle) = (job.job_id, job.handle);
		Some(match job.ty {
			io::Job::READ => Self::Read {
				job_id,
				handle,
				length: u64::from_ne_bytes(data.try_into().ok()?),
			},
			io::Job::PEEK => Self::Peek {
				job_id,
				handle,
				length: u64::from_ne_bytes(data.try_into().ok()?),
			},
			io::Job::WRITE => Self::Write {
				job_id,
				handle,
				data,
			},
			io::Job::OPEN => Self::Open {
				job_id,
				handle,
				path: data,
			},
			io::Job::CREATE => Self::Create {
				job_id,
				handle,
				path: data,
			},
			io::Job::CLOSE => Self::Close { handle },
			io::Job::SEEK => {
				let offt = u64::from_ne_bytes(data.try_into().ok()?);
				Self::Seek {
					job_id,
					handle,
					from: io::SeekFrom::try_from_raw(job.from_anchor, offt).ok()?,
				}
			}
			_ => return None,
		})
	}

	with!(buf reply_read = READ, data);
	with!(buf reply_peek = PEEK, data);
	with!(handle reply_open = OPEN, path);
	with!(handle reply_create = CREATE, path);
	with!(u64 reply_write = WRITE, amount);
	with!(u64 reply_seek = SEEK, position);

	pub fn reply_error(buf: &mut Vec<u8>, job_id: u32, error: Error) -> Result<(), ()> {
		buf.extend(
			io::Job {
				job_id,
				result: error as _,
				..Default::default()
			}
			.as_ref(),
		);
		Ok(())
	}
}
