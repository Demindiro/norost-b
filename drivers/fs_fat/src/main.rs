#![feature(norostb)]
// FIXME figure out why rustc doesn't let us use data structures from an re-exported crate in
// stdlib
#![feature(rustc_private)]

use norostb_kernel::syscall;
use std::fs;
use std::io::{Read, Seek, Write};
use std::ptr::NonNull;

fn main() {
	// TODO get disk from arguments

	let disk = fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("virtio-blk/disk/0")
		.expect("failed to open disk");

	let disk = driver_utils::io::Monitor::new(disk, driver_utils::io::monitor::log_stderr);
	let disk = driver_utils::io::BufBlock::new(disk);
	let fs =
		fatfs::FileSystem::new(disk, fatfs::FsOptions::new()).expect("failed to open filesystem");

	dbg!(fs.stats());

	// Register new table of Streaming type
	let tbl = syscall::create_table(b"fat", syscall::TableType::Streaming).unwrap();

	let mut queries = driver_utils::Arena::new();
	let mut open_files = driver_utils::Arena::new();

	let mut buf = [0; 4096];
	let buf = &mut buf;

	loop {
		// Wait for events from the table
		let mut job = std::os::norostb::Job::default();
		job.buffer = NonNull::new(buf.as_mut_ptr());
		job.buffer_size = buf.len().try_into().unwrap();
		if std::os::norostb::take_job(tbl, &mut job).is_err() {
			std::thread::sleep(std::time::Duration::from_millis(100));
			continue;
		}

		match job.ty {
			syscall::Job::OPEN => {
				let path = std::str::from_utf8(&buf[..job.operation_size.try_into().unwrap()])
					.expect("what do?");
				dbg!(path);
				if fs.root_dir().open_file(path).is_ok() {
					job.handle = open_files.insert((path.to_string(), 0u64));
				} else {
					match fs.root_dir().open_file(path) {
						Ok(_) => unreachable!(),
						Err(e) => dbg!(e),
					};
					todo!("how do I return an error?");
				}
			}
			syscall::Job::CREATE => {
				todo!()
			}
			syscall::Job::READ => {
				let (path, offset) = &open_files[job.handle];
				let mut file = fs.root_dir().open_file(path).unwrap();
				file.seek(std::io::SeekFrom::Start(*offset)).unwrap();
				let l = file
					.read(&mut buf[..job.operation_size.try_into().unwrap()])
					.unwrap();
				job.operation_size = l.try_into().unwrap();
			}
			syscall::Job::WRITE => {
				todo!()
			}
			syscall::Job::QUERY => {
				let entries = fs
					.root_dir()
					.iter()
					.filter_map(|e| e.ok().map(|e| e.file_name()))
					.collect::<Vec<_>>();
				job.query_id = queries.insert(entries);
			}
			syscall::Job::QUERY_NEXT => {
				match queries[job.query_id].pop() {
					Some(f) => {
						buf[..f.len()].copy_from_slice(f.as_bytes());
						job.operation_size = f.len().try_into().unwrap();
					}
					None => {
						queries.remove(job.query_id);
						job.operation_size = 0;
					}
				};
			}
			syscall::Job::SEEK => {
				todo!()
			}
			t => todo!("job type {}", t),
		}

		std::os::norostb::finish_job(tbl, &job).unwrap();
	}
}
