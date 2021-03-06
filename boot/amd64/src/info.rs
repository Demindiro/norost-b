use core::mem::MaybeUninit;

#[repr(C)]
#[repr(align(8))]
pub struct Info {
	pub memory_regions_offset: u16,
	pub memory_regions_len: u16,
	pub initfs_ptr: u32,
	pub initfs_len: u32,
	// Ensure rsdp has 64 bit alignment.
	pub _padding: u32,
	pub rsdp: MaybeUninit<rsdp::Rsdp>,
}

#[derive(Clone, Copy)]
#[repr(C)]
#[repr(align(8))]
pub struct MemoryRegion {
	pub base: u64,
	pub size: u64,
}
