use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::io::{
	FromRawFd,
};
use std::ptr;

use libc::{
	MAP_SHARED,
	O_CLOEXEC,
	O_RDONLY,
	O_RDWR,
	O_SYNC,
	PROT_READ,
	PROT_WRITE,
	c_void,
	mmap,
	munmap,
	open,
};

use super::PciEndpoint;

#[derive(Debug)]
pub struct Mapped {
	ptr: ptr::NonNull<u8>, // u8 instead of void for easier offset operations
	len: usize,
	endpoint: PciEndpoint,
}

impl Drop for Mapped {
	fn drop(&mut self) {
		unsafe {
			let res = munmap(
				self.ptr.as_ptr() as *mut c_void,
				self.len,
			);
			if 0 != res {
				panic!("munmap failed: {}", io::Error::last_os_error());
			}
		}
	}
}

impl Mapped {
	pub fn endpoint(&self) -> PciEndpoint {
		self.endpoint
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn read_byte(&self, offset: usize) -> u8 {
		assert!(offset < self.len);
		unsafe { ptr::read(self.ptr.as_ptr().add(offset)) }
	}

	pub fn read_dword(&self, offset: usize) -> u32 {
		assert!(offset & 3 == 0);
		assert!(offset + 3 < self.len);
		u32::from_le(unsafe { ptr::read(self.ptr.as_ptr().add(offset) as *const u32) })
	}

	pub fn read_slice(&self, offset: usize, target: &mut [u8]) {
		if target.is_empty() { return; }
		assert!(offset < self.len);
		assert!(target.len() <= self.len - offset);
		unsafe {
			ptr::copy_nonoverlapping(
				self.ptr.as_ptr().add(offset),
				target.as_mut_ptr(),
				target.len(),
			)
		}
	}

	pub fn read_into_vec(&self) -> Vec<u8> {
		let mut v = Vec::with_capacity(self.len);
		unsafe {
			v.set_len(self.len);
			self.read_slice(0, &mut v[..self.len]);
		}
		v
	}

	pub fn write_byte(&mut self, offset: usize, data: u8) {
		assert!(offset < self.len);
		unsafe { ptr::write(self.ptr.as_ptr().add(offset), data) }
	}

	pub fn write_dword(&mut self, offset: usize, data: u32) {
		assert!(offset & 3 == 0);
		assert!(offset + 3 < self.len);
		unsafe { ptr::write(self.ptr.as_ptr().add(offset) as *mut u32, data.to_le()) }
	}
}

// TODO: exclusive open / file locking?
pub fn inner_open(endpoint: PciEndpoint, path: String, writable: bool) -> io::Result<Mapped> {
	let open_flags = if writable { O_RDWR } else { O_RDONLY } | O_CLOEXEC | O_SYNC;
	let mmap_prot_flags = if writable { PROT_WRITE } else { 0 } | PROT_READ;

	let path = CString::new(path)?;
	
	let fd = unsafe { open(path.as_ptr(), open_flags) };
	if -1 == fd {
		return Err(io::Error::last_os_error());
	}
	// now get fd managed to prevent resource leak
	let f = unsafe { fs::File::from_raw_fd(fd) };

	let size = f.metadata()?.len();
	assert!(size < !0usize as u64);
	let size = size as usize;
	let area = unsafe {
		mmap(
			ptr::null_mut(),
			size,
			mmap_prot_flags,
			MAP_SHARED,
			fd,
			0,
		)
	};

	if area as usize == !0usize {
		return Err(io::Error::last_os_error());
	}
	match ptr::NonNull::new(area as *mut u8) {
		None => panic!("mmap shouldn't return NULL ever"),
		Some(area) => Ok(Mapped{
			ptr: area,
			len: size,
			endpoint,
		}),
	}
}
