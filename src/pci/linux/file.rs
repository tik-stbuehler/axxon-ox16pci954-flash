use std::fs;
use std::io;
use std::os::unix::fs::FileExt;

use crate::pci::PciEndpoint;

/* PCI is always little endian */

fn pci_dword_to_bytes(val: u32) -> [u8; 4] {
/*
	use std::mem::transmute;
	let val = val.to_le();
	unsafe { transmute::<u32, [u8; 4]>(val) }
*/
	[
		val as u8,
		(val >> 8) as u8,
		(val >> 16) as u8,
		(val >> 24) as u8,
	]
}

fn pci_dword_from_bytes(val: [u8; 4]) -> u32 {
/*
	use std::mem::transmute;
	let val = unsafe { transmute::<[u8; 4], u32>(val) };
	u32::from_le(val)
*/
	(val[0] as u32)
	| (val[1] as u32) << 8
	| (val[2] as u32) << 16
	| (val[3] as u32) << 24
}

pub struct File {
	file: fs::File,
	len: usize,
	endpoint: PciEndpoint,
}

impl File {
	pub fn endpoint(&self) -> PciEndpoint {
		self.endpoint
	}

	pub fn len(&self) -> usize {
		self.len
	}

	fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
		// reading should get all data in one step (in this case)
		let l = self.file.read_at(buf, offset)?;
		if l != buf.len() {
			Err(io::Error::new(io::ErrorKind::UnexpectedEof, "failed to fill whole buffer"))
		} else {
			Ok(())
		}
	}

	fn write_exact_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
		// writing should push all data in one step (in this case)
		let l = self.file.write_at(buf, offset)?;
		if l != buf.len() {
			Err(io::Error::new(io::ErrorKind::Other, "failed to write whole buffer"))
		} else {
			Ok(())
		}
	}

	pub fn read_byte(&self, offset: usize) -> u8 {
		assert!(offset < self.len);
		let mut buf = [0u8];
		self.read_exact_at(&mut buf, offset as u64).expect("read within length must not fail");
		buf[0]
	}

	pub fn read_dword(&self, offset: usize) -> u32 {
		assert!(offset & 3 == 0);
		assert!(offset + 3 < self.len);

		let mut buf = [0u8; 4];
		self.read_exact_at(&mut buf, offset as u64).expect("read within length must not fail");
		pci_dword_from_bytes(buf)
	}

	pub fn read_slice(&self, offset: usize, target: &mut [u8]) {
		if target.is_empty() { return; }
		assert!(offset < self.len);
		assert!(target.len() <= self.len - offset);

		self.read_exact_at(target, offset as u64).expect("read within length must not fail");
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
		self.write_exact_at(&[data], offset as u64).expect("write withing length must not fail")
	}

	pub fn write_dword(&mut self, offset: usize, data: u32) {
		assert!(offset & 3 == 0);
		assert!(offset + 3 < self.len);
		let buf = pci_dword_to_bytes(data);
		self.write_exact_at(&buf, offset as u64).expect("write withing length must not fail")
	}
}

// TODO: exclusive open / file locking?
pub fn inner_open(endpoint: PciEndpoint, path: String, writable: bool) -> io::Result<File> {
	let file = fs::OpenOptions::new()
		.read(true)
		.write(writable)
		.open(path)?;

	let size = file.metadata()?.len();
	assert!(size < !0usize as u64);
	let len = size as usize;

	Ok(File {
		file,
		len,
		endpoint,
	})
}
