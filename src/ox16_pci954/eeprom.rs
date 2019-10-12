use std::io;

use crate::pci::{
	PciEndpoint,
	PciResource,
	open_resource_readwrite,
};

use crate::serial::{
	Hardware,
	HardwareOperations,
	OutPins,
};

struct WrapPciResource<R>
where
	R: PciResource,
{
	resource: R,
}

impl<R> Hardware for WrapPciResource<R>
where
	R: PciResource,
{
	fn set_pins(&mut self, pins: OutPins) {
		let clk = if pins.clock { 0x01 } else { 0x00 };
		let cs = if pins.chip_select { 0x02 } else { 0x00 };
		let data = if pins.data { 0x04 } else { 0x00 };
		// println!("EEPROM out: {:02x}", clk | cs | data);
		self.resource.write_byte(3usize, clk | cs | data);
	}

	fn read_pin(&mut self) -> bool {
		let input = self.resource.read_byte(3usize);
		// println!("EEPROM in: {:02x}", input);
		0 != (input & 0x08)
	}
}

pub fn open_eeprom(ep: PciEndpoint) -> io::Result<impl HardwareOperations> {
	let resource = open_resource_readwrite(ep, 3)?;
	Ok(WrapPciResource{resource})
}
