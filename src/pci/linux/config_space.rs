use super::File;
use crate::pci::{
	PciEndpoint,
	config_space,
};

impl config_space::PciConfigSpaceReadOnly for File {
	fn endpoint(&self) -> PciEndpoint {
		File::endpoint(self)
	}

	fn len(&self) -> usize {
		File::len(self)
	}

	fn read_byte(&self, offset: usize) -> u8 {
		File::read_byte(self, offset)
	}

	fn read_dword(&self, offset: usize) -> u32 {
		File::read_dword(self, offset)
	}

	fn read_slice(&self, offset: usize, target: &mut [u8]) {
		File::read_slice(self, offset, target)
	}

	fn read_into_vec(&self) -> Vec<u8> {
		File::read_into_vec(self)
	}
}

impl config_space::PciConfigSpace for File {
	fn write_byte(&mut self, offset: usize, data: u8) {
		File::write_byte(self, offset, data)
	}

	fn write_dword(&mut self, offset: usize, data: u32) {
		File::write_dword(self, offset, data)
	}
}
