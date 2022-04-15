#![feature(norostb)]
// FIXME figure out why rustc doesn't let us use data structures from an re-exported crate in
// stdlib
#![feature(rustc_private)]

use core::ptr::NonNull;
use norostb_kernel::{self as kernel, io::Job, syscall};
use virtio_block::Sector;

fn main() {
	println!("Hello, world! from Rust");

	let mut buf = [0; 256];
	let mut obj = std::os::norostb::ObjectInfo::new(&mut buf);

	let mut id = None;
	let mut dev = None;
	println!("iter tables");
	'found_dev: while let Some((i, inf)) = syscall::next_table(id) {
		println!("table: {:?} -> {:?}", i, core::str::from_utf8(inf.name()));
		if inf.name() == b"pci" {
			let tags = b"vendor-id:1af4&device-id:1001";
			let h = std::os::norostb::query(i, tags).unwrap();
			println!("{:?}", h);
			while let Ok(true) = std::os::norostb::query_next(h, &mut obj) {
				println!("{:#?}", &obj);
				dev = Some((i, &buf[..obj.path_len]));
				break 'found_dev;
			}
		}
		id = Some(i);
	}

	let (tbl, dev) = dev.unwrap();
	dbg!(core::str::from_utf8(dev));
	let handle = std::os::norostb::open(tbl, dev).unwrap();

	let pci_config = NonNull::new(0x1000_0000 as *mut _);
	let pci_config = syscall::map_object(handle, pci_config, 0, usize::MAX).unwrap();

	println!("handle: {:?}", handle);

	let pci = unsafe { pci::Pci::new(pci_config.cast(), 0, 0, &[]) };

	let mut dma_addr = 0x2666_0000;

	let mut dev = {
		let h = pci.get(0, 0, 0).unwrap();
		match h {
			pci::Header::H0(h) => {
				for (i, b) in h.base_address.iter().enumerate() {
					println!("{}: {:x}", i, b.get());
				}

				let mut map_addr = 0x2000_0000 as *mut kernel::Page;

				let get_phys_addr = |addr| {
					let addr = NonNull::new(addr as *mut _).unwrap();
					syscall::physical_address(addr).unwrap()
				};
				let map_bar = |bar: u8| {
					let addr = map_addr.cast();
					syscall::map_object(handle, NonNull::new(addr), (bar + 1).into(), usize::MAX)
						.unwrap();
					map_addr = map_addr.wrapping_add(16);
					NonNull::new(addr as *mut _).unwrap()
				};
				let dma_alloc = |size| {
					println!("dma: {:#x}", dma_addr);
					let d = core::ptr::NonNull::new(dma_addr as *mut _).unwrap();
					let res = syscall::alloc_dma(Some(d), size).unwrap();
					dma_addr += res;
					let a = syscall::physical_address(d).unwrap();
					Ok((d.cast(), a))
				};
				let d =
					virtio_block::BlockDevice::new(h, get_phys_addr, map_bar, dma_alloc).unwrap();

				println!("pci status: {:#x}", h.status());

				d
			}
			_ => unreachable!(),
		}
	};

	// Register new table of Streaming type
	let tbl = syscall::create_table(b"virtio-blk", syscall::TableType::Streaming).unwrap();

	let mut sectors = [Sector::default(); 8];

	let mut queries = driver_utils::Arena::new();
	let mut data_handles = driver_utils::Arena::new();

	let mut buf = [0; 4096];
	let buf = &mut buf;

	enum QueryState {
		Data,
		Info,
	}

	loop {
		// Wait for events from the table
		let mut job = std::os::norostb::Job::default();
		job.buffer = NonNull::new(buf.as_mut_ptr());
		job.buffer_size = buf.len().try_into().unwrap();
		if std::os::norostb::take_job(tbl, &mut job).is_err() {
			std::thread::sleep(std::time::Duration::from_millis(10));
			continue;
		}

		match job.ty {
			Job::OPEN => {
				job.handle = data_handles.insert(0);
			}
			Job::READ => {
				let offset = data_handles[job.handle];
				let sector = offset / u64::try_from(Sector::SIZE).unwrap();
				let offset = offset % u64::try_from(Sector::SIZE).unwrap();
				let offset = u16::try_from(offset).unwrap();

				dev.read(&mut sectors, sector, || {
					std::thread::sleep(std::time::Duration::from_millis(1));
				})
				.unwrap();

				let size = job.operation_size.min(
					(Sector::slice_as_u8(&sectors).len() - usize::from(offset))
						.try_into()
						.unwrap(),
				);

				job.operation_size = size;
				data_handles[job.handle] = u64::from(offset) + u64::from(job.operation_size);

				let size = usize::try_from(size).unwrap();
				let offset = usize::from(offset);
				buf[..size].copy_from_slice(&Sector::slice_as_u8(&sectors)[offset..][..size]);
			}
			Job::WRITE => {
				let offset = data_handles[job.handle];
				let sector = offset / u64::try_from(Sector::SIZE).unwrap();
				let offset = offset % u64::try_from(Sector::SIZE).unwrap();
				let offset = u16::try_from(offset).unwrap();

				dev.read(&mut sectors, sector, || {
					std::thread::sleep(std::time::Duration::from_millis(1));
				})
				.unwrap();

				let data = &buf[..job.operation_size as usize];
				Sector::slice_as_u8_mut(&mut sectors)[offset.into()..][..data.len()]
					.copy_from_slice(data);

				dev.write(&sectors, sector, || {
					std::thread::sleep(std::time::Duration::from_millis(1));
				})
				.unwrap();

				data_handles[job.handle] = u64::from(offset) + u64::from(job.operation_size);
			}
			Job::QUERY => {
				job.handle = queries.insert(Some(QueryState::Data));
			}
			Job::QUERY_NEXT => {
				match queries[job.handle] {
					Some(QueryState::Data) => {
						buf[..4].copy_from_slice(b"data");
						job.operation_size = 4;
						queries[job.handle] = Some(QueryState::Info);
					}
					Some(QueryState::Info) => {
						buf[..4].copy_from_slice(b"info");
						job.operation_size = 4;
						queries[job.handle] = None;
					}
					None => {
						queries.remove(job.handle);
						job.operation_size = 0;
					}
				};
			}
			Job::SEEK => {
				use norostb_kernel::io::SeekFrom;
				let from = SeekFrom::try_from_raw(job.from_anchor, job.from_offset).unwrap();
				let offset = match from {
					SeekFrom::Start(n) => n,
					_ => todo!(),
				};
				data_handles[job.handle] = offset;
			}
			t => todo!("job type {}", t),
		}

		std::os::norostb::finish_job(tbl, &job).unwrap();
	}
}
