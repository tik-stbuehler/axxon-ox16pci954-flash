use super::PciEndpoint;

pub trait PciConfigSpaceReadOnly {
	fn endpoint(&self) -> PciEndpoint;
	fn len(&self) -> usize;

	fn read_byte(&self, offset: usize) -> u8;
	fn read_dword(&self, offset: usize) -> u32; // handle PCI little-endian conversion
	fn read_slice(&self, offset: usize, target: &mut [u8]);
	fn read_into_vec(&self) -> Vec<u8>;
}

pub trait PciConfigSpace: PciConfigSpaceReadOnly {
	fn write_byte(&mut self, offset: usize, data: u8);
	fn write_dword(&mut self, offset: usize, data: u32); // handle PCI little-endian conversion
}

impl<'a, S: ?Sized + PciConfigSpaceReadOnly> PciConfigSpaceReadOnly for &'a mut S {
	fn endpoint(&self) -> PciEndpoint {
		S::endpoint(*self)
	}
	fn len(&self) -> usize {
		S::len(*self)
	}

	fn read_byte(&self, offset: usize) -> u8 {
		S::read_byte(*self, offset)
	}
	fn read_dword(&self, offset: usize) -> u32 {
		S::read_dword(*self, offset)
	}
	fn read_slice(&self, offset: usize, target: &mut [u8]) {
		S::read_slice(*self, offset, target)
	}
	fn read_into_vec(&self) -> Vec<u8> {
		S::read_into_vec(*self)
	}
}

impl<'a, S: ?Sized + PciConfigSpace> PciConfigSpace for &'a mut S {
	fn write_byte(&mut self, offset: usize, data: u8) {
		S::write_byte(*self, offset, data);
	}
	fn write_dword(&mut self, offset: usize, data: u32) {
		S::write_dword(*self, offset, data);
	}
}
