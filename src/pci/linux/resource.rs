use super::Mapped;
use crate::pci::{
	PciEndpoint,
	resource,
};

impl resource::PciResourceReadOnly for Mapped {
	fn endpoint(&self) -> PciEndpoint {
		Mapped::endpoint(self)
	}

	fn len(&self) -> usize {
		Mapped::len(self)
	}

	fn read_byte(&self, offset: usize) -> u8 {
		Mapped::read_byte(self, offset)
	}

	fn read_dword(&self, offset: usize) -> u32 {
		Mapped::read_dword(self, offset)
	}

	fn read_slice(&self, offset: usize, target: &mut [u8]) {
		Mapped::read_slice(self, offset, target)
	}

	fn read_into_vec(&self) -> Vec<u8> {
		Mapped::read_into_vec(self)
	}
}

impl resource::PciResource for Mapped {
	fn write_byte(&mut self, offset: usize, data: u8) {
		Mapped::write_byte(self, offset, data)
	}

	fn write_dword(&mut self, offset: usize, data: u32) {
		Mapped::write_dword(self, offset, data)
	}
}
