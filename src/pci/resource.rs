use super::PciEndpoint;

pub trait PciResourceReadOnly {
	fn endpoint(&self) -> PciEndpoint;
	fn len(&self) -> usize;

	fn read_byte(&self, offset: usize) -> u8;
	fn read_dword(&self, offset: usize) -> u32; // handle PCI little-endian conversion
	fn read_slice(&self, offset: usize, target: &mut [u8]);
	fn read_into_vec(&self) -> Vec<u8>;
}

pub trait PciResource: PciResourceReadOnly {
	fn write_byte(&mut self, offset: usize, data: u8);
	fn write_dword(&mut self, offset: usize, data: u32); // handle PCI little-endian conversion
}

impl<'a, R: ?Sized + PciResourceReadOnly> PciResourceReadOnly for &'a mut R {
	fn endpoint(&self) -> PciEndpoint {
		R::endpoint(*self)
	}
	fn len(&self) -> usize {
		R::len(*self)
	}

	fn read_byte(&self, offset: usize) -> u8 {
		R::read_byte(*self, offset)
	}
	fn read_dword(&self, offset: usize) -> u32 {
		R::read_dword(*self, offset)
	}
	fn read_slice(&self, offset: usize, target: &mut [u8]) {
		R::read_slice(*self, offset, target)
	}
	fn read_into_vec(&self) -> Vec<u8> {
		R::read_into_vec(*self)
	}
}

impl<'a, R: ?Sized + PciResource> PciResource for &'a mut R {
	fn write_byte(&mut self, offset: usize, data: u8) {
		R::write_byte(*self, offset, data);
	}
	fn write_dword(&mut self, offset: usize, data: u32) {
		R::write_dword(*self, offset, data);
	}
}
