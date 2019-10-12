/* reverse engineered */

/* Chip documentation: https://www.broadcom.com/products/pcie-switches-bridges/pcie-bridges/pex8112#documentation */

use std::io;

use crate::pci::{
	PciConfigSpace,
	PciEndpoint,
};

mod eectl;
mod image;

pub use self::image::IMAGE;

#[allow(dead_code)]
mod consts {
	pub const MAIN_CONTROL_REGISTER_INDEX: usize = 0x84; // "MAININDEX"
	pub const MAIN_CONTROL_REGISTER_DATA: usize = 0x88; // "MAINDATA"

	// addresses for MAIN_CONTROL_REGISTER_INDEX
	pub const DEVICE_INITIALIZATION: u32 = 0x00; // "DEVINIT"
	pub const SERIAL_EEPROM_CONTROL: u32 = 0x04; // "EECTL"

	// EEPROM commands:
	// WRSR: 0x01
	pub const WRITE_EE_OPCODE:       u8 = 0x02; // write data; clears WEL when CS gets disabled
	pub const READ_EE_OPCODE:        u8 = 0x03; // read data
	pub const WRDI_EE_OPCODE:        u8 = 0x04; // write disable (clears WEL)
	pub const READ_STATUS_EE_OPCODE: u8 = 0x05; // "RDSR"
	pub const WREN_EE_OPCODE:        u8 = 0x06; // write enable (sets WEL)

	// offset in EEPROM to write "axxon" to
	pub const EEPROM_SIGNATURE_OFFSET: usize = 0x78;
}

use self::consts::*;
use self::eectl::*;

trait PciConfigSpaceEeExt: PciConfigSpace {
	fn main_read(&mut self, offset: u32) -> u32 {
		self.write_dword(MAIN_CONTROL_REGISTER_INDEX, offset);
		self.read_dword(MAIN_CONTROL_REGISTER_DATA)
	}

	fn main_write(&mut self, offset: u32, data: u32) {
		self.write_dword(MAIN_CONTROL_REGISTER_INDEX, offset);
		self.write_dword(MAIN_CONTROL_REGISTER_DATA, data);
	}

	fn eectl_read(&mut self) -> EeControlRead {
		let data = EeControlRead(self.main_read(SERIAL_EEPROM_CONTROL));
		// TODO: debug log
		// eprintln!("EECTL read : {:?}", data);
		data
	}

	fn eectl_write(&mut self, data: EeControlWrite) {
		// TODO: debug log
		// eprintln!("EECTL write: {:?}", data);
		self.main_write(SERIAL_EEPROM_CONTROL, data.0);
	}

	/// returns EECTL if reached a idle state; returns error on timeout
	fn ee_waitidle(&mut self) -> crate::AResult<EeControlRead> {
		for _ in 0..0xffff {
			let eectl = self.eectl_read();
			if !eectl.is_busy() {
				return Ok(eectl)
			}
		}
		bail!("EEPROM Timeout error - always busy!");
	}

	fn ee_off(&mut self) -> crate::AResult<()> {
		let eectl = self.ee_waitidle()?;
		if !eectl.is_off() {
			self.eectl_write(EeControlWrite::off());
		}
		Ok(())
	}

	fn ee_sendbyte(&mut self, data: u8) -> crate::AResult<()> {
		self.ee_waitidle()?;
		self.eectl_write(EeControlWrite::write_data(data));
		Ok(())
	}

	fn ee_readbyte(&mut self) -> crate::AResult<u8> {
		self.ee_waitidle()?;
		self.eectl_write(EeControlWrite::read_data());
		Ok(self.ee_waitidle()?.data())
	}
}
impl<S: PciConfigSpace+?Sized> PciConfigSpaceEeExt for S {}

pub struct Flash<S: PciConfigSpace> {
	space: S,
	address_width: AddressWidth,
}

impl<S: PciConfigSpace> Flash<S> {
	fn send_address(&mut self, address: usize) -> crate::AResult<()> {
		match self.address_width {
			AddressWidth::One => {
				self.space.ee_sendbyte(address as u8)?;
			},
			AddressWidth::Two => {
				self.space.ee_sendbyte((address >> 8) as u8)?;
				self.space.ee_sendbyte(address as u8)?;
			},
			AddressWidth::Three => {
				self.space.ee_sendbyte((address >> 16) as u8)?;
				self.space.ee_sendbyte((address >> 8) as u8)?;
				self.space.ee_sendbyte(address as u8)?;
			},
		}
		Ok(())
	}

	pub fn readstatus(&mut self) -> crate::AResult<u8> {
		self.space.ee_off()?;
		self.space.ee_sendbyte(READ_STATUS_EE_OPCODE)?;
		let result = self.space.ee_readbyte()?;
		self.space.ee_off()?;
		Ok(result)
	}

	pub fn writer<'a>(&'a mut self, address: usize) -> crate::AResult<FlashWriter<'a, S>> {
		self.space.ee_off()?;
		self.space.ee_sendbyte(WREN_EE_OPCODE)?;
		self.space.ee_off()?;
		self.space.ee_sendbyte(WRITE_EE_OPCODE)?;
		self.send_address(address)?;

		Ok(FlashWriter { flash: self })
	}

	pub fn write_byte(&mut self, address: usize, data: u8) -> crate::AResult<()> {
		self.writer(address)?.write_byte(data)
	}

	pub fn reader<'a>(&'a mut self, address: usize) -> crate::AResult<FlashReader<'a, S>> {
		self.space.ee_off()?;
		self.space.ee_sendbyte(READ_EE_OPCODE)?;
		self.send_address(address)?;
		Ok(FlashReader { flash: self })
	}

	pub fn read_byte(&mut self, address: usize) -> crate::AResult<u8> {
		self.reader(address)?.read_byte()
	}

	pub fn read_signature(&mut self) -> crate::AResult<[u8; 5]> {
		let mut signature = [0u8; 5]; // expecting b"axxon"
		self.reader(EEPROM_SIGNATURE_OFFSET)?.read(&mut signature)?;
		Ok(signature)
	}

	pub fn verify_signature(&mut self) -> crate::AResult<()> {
		const AXXON_SIGNATURE: [u8; 5] = *b"axxon";
		let signature = self.read_signature()?;
		ensure!(signature == AXXON_SIGNATURE, "Unexpected signature: {:?} (expected: {:?})", signature, AXXON_SIGNATURE);
		Ok(())
	}
}

pub struct FlashReader<'a, S: PciConfigSpace + 'a> {
	flash: &'a mut Flash<S>,
}

impl<'a, S: PciConfigSpace> FlashReader<'a, S> {
	pub fn read_byte(&mut self) -> crate::AResult<u8> {
		self.flash.space.ee_readbyte()
	}

	pub fn read(&mut self, target: &mut [u8]) -> crate::AResult<()> {
		for t in target.iter_mut() {
			*t = self.read_byte()?;
		}
		Ok(())
	}
}

impl<'a, S: PciConfigSpace> Drop for FlashReader<'a, S> {
	fn drop(&mut self) {
		let _ = self.flash.space.ee_off();
	}
}

impl<'a, S: PciConfigSpace> Iterator for FlashReader<'a, S> {
	type Item = crate::AResult<u8>;

	fn next(&mut self) -> Option<Self::Item> {
		Some(self.read_byte())
	}
}

pub struct FlashWriter<'a, S: PciConfigSpace + 'a> {
	flash: &'a mut Flash<S>,
}

impl<'a, S: PciConfigSpace> FlashWriter<'a, S> {
	pub fn write_byte(&mut self, data: u8) -> crate::AResult<()> {
		self.flash.space.ee_sendbyte(data)
	}

	pub fn write(&mut self, data: &[u8]) -> crate::AResult<()> {
		for b in data {
			self.flash.space.ee_sendbyte(*b)?;
		}
		Ok(())
	}
}

impl<'a, S: PciConfigSpace> Drop for FlashWriter<'a, S> {
	fn drop(&mut self) {
		let _ = self.flash.space.ee_off();
	}
}

impl<'a, S: PciConfigSpace> io::Write for FlashWriter<'a, S> {
	fn write(&mut self, data: &[u8]) -> io::Result<usize> {
		FlashWriter::write(self, data).map_err(|e| {
			io::Error::new(io::ErrorKind::Other, format!("{:?}", e))
		})?;
		Ok(data.len())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

pub fn write_image<S: PciConfigSpace>(flash: &mut Flash<S>, image: &[u8]) -> crate::AResult<()> {
	assert!(image.len() <= EEPROM_SIGNATURE_OFFSET);

	// write image and padding
	{
		let mut writer = flash.writer(0)?;
		for b in image {
			writer.write_byte(*b)?;
		}
		for _ in image.len()..EEPROM_SIGNATURE_OFFSET {
			writer.write_byte(0xff)?;
		}
	}

	// verify
	{
		let mut reader = flash.reader(0)?;
		for address in 0..image.len() {
			let d = reader.read_byte()?;
			ensure!(d == image[address],
				"Verify failed at {:02x}: expected {:04x}, flash is {:04x}", address, image[address], d
			);
		}
	}

	// "signature"
	flash.writer(EEPROM_SIGNATURE_OFFSET)?.write(b"axxon")?;

	Ok(())
}

pub fn extract_image<S: PciConfigSpace>(flash: &mut Flash<S>) -> crate::AResult<Vec<u8>> {
	let mut reader = flash.reader(0)?;
	let mut buf = Vec::new();

	let magic = reader.read_byte()?;
	ensure!(0x5a == magic, "Invalid image (first byte: 0x{:02x})", magic);
	buf.push(magic);

	let flags = reader.read_byte()?;
	ensure!(0 == (flags & !0x3), "Invalid image (second byte: 0x{:02x})", flags);
	buf.push(flags);

	let reg_count_lo = reader.read_byte()?;
	let reg_count_hi = reader.read_byte()?;
	let reg_count = (reg_count_lo as usize) + ((reg_count_hi as usize) << 8);
	ensure!(0 == reg_count % 6, "Invalid size of register byte count: {}", reg_count);
	buf.push(reg_count_lo);
	buf.push(reg_count_hi);
	for _ in 0..reg_count {
		buf.push(reader.read_byte()?);
	}

	let mem_count_lo = reader.read_byte()?;
	let mem_count_hi = reader.read_byte()?;
	let mem_count = (mem_count_lo as usize) + ((mem_count_hi as usize) << 8);
	ensure!(0 == mem_count % 4, "Invalid size of shared memory byte count: {}", mem_count);
	buf.push(mem_count_lo);
	buf.push(mem_count_hi);
	for _ in 0..mem_count {
		buf.push(reader.read_byte()?);
	}

	Ok(buf)
}

pub fn open_flash<S: PciConfigSpace>(mut space: S) -> crate::AResult<Flash<S>> {
	// "PCI Base Address 0 Enable"
	// BAR0 is likely hardwired to be enabled, also we don't use it.
	// space.write_byte(0x48, 0x02);

	let eectl = space.eectl_read();
	ensure!(eectl.is_present(), "No EEPROM present");
	ensure!(eectl.is_valid(), "EEPROM invalid");
	let address_width = match eectl.address_width() {
		None => bail!("EEPROM address width unknown"),
		Some(aw) => aw,
	};

	let device_flags = space.main_read(DEVICE_INITIALIZATION);
	let pci_express_enabled = 0 != (device_flags & 0b1_0000);
	ensure!(pci_express_enabled, "PCI Express not enabled");
	let pci_enabled = 0 != (device_flags & 0b10_0000);
	ensure!(pci_enabled, "PCI not enabled");
	let frequency = device_flags & 0b1111;
	// why would we care?
	ensure!(frequency == 0b0011, "Speed not default (33.3/66/62.5)");

	let mut flash = Flash {
		space,
		address_width,
	};

	flash.verify_signature()?;

	Ok(flash)
}

pub fn detect_address_width<S: PciConfigSpace>(space: &mut S) -> crate::AResult<AddressWidth> {
	// - while waiting for address bytes the EEPROM should emit 0xff data bytes;
	// - `ee_readbyte` should send zeroes while reading
	// - so if we call `ee_readbyte` before sending the full address it should
	//   send a zero address and return 0xff, until the address is complete and
	//   it returns real data (which still might be 0xff!)

	space.ee_off()?;
	space.ee_sendbyte(READ_EE_OPCODE)?;
	// should have at least one address byte
	ensure!(space.ee_readbyte()? == 0xff, "First data byte with empty address is not 0xff");
	if space.ee_readbyte()? != 0xff {
		// one address byte
		return Ok(AddressWidth::One);
	}
	// assuming "zero" address bytes, the next received byte would be at `addr`:
	let mut addr: usize = 2;
	let mut data: u8;
	loop {
		data = space.ee_readbyte()?;
		if data != 0xff { break; }
		addr += 1;
		ensure!(addr < 0x200, "Couldn't find a non-0xff byte, may need to test address width by writing data");
	}

	if addr < 0x101 {
		// try one address byte
		let addr = addr - 1; // first byte was waiting for address
		space.ee_off()?;
		space.ee_sendbyte(READ_EE_OPCODE)?;
		space.ee_sendbyte(addr as u8)?;
		// if address width is longer than a byte we should still get 0xff here,
		// but data is not 0xff!
		if data == space.ee_readbyte()? {
			space.ee_off()?;
			return Ok(AddressWidth::One);
		}
	}

	if addr < 0x1_0002 {
		// try two address bytes
		let addr = addr - 2; // first two bytes were waiting for address
		space.ee_off()?;
		space.ee_sendbyte(READ_EE_OPCODE)?;
		space.ee_sendbyte((addr >> 8) as u8)?;
		space.ee_sendbyte(addr as u8)?;
		// if address width is longer than two bytes we should still get 0xff here,
		// but data is not 0xff!
		if data == space.ee_readbyte()? {
			space.ee_off()?;
			return Ok(AddressWidth::Two);
		}
	}

	if addr < 0x100_0003 {
		// try three address bytes
		let addr = addr - 3; // first three bytes were waiting for address
		space.ee_off()?;
		space.ee_sendbyte(READ_EE_OPCODE)?;
		space.ee_sendbyte((addr >> 16) as u8)?;
		space.ee_sendbyte((addr >> 8) as u8)?;
		space.ee_sendbyte(addr as u8)?;
		// if address width is longer than three bytes we should still get 0xff here,
		// but data is not 0xff!
		if data == space.ee_readbyte()? {
			space.ee_off()?;
			return Ok(AddressWidth::Two);
		}
	}

	space.ee_off()?;

	bail!("Couldn't find address width, even we found a non-0xff byte on read {}", addr);
}

pub fn detect_address_width_allow_writing<S: PciConfigSpace>(mut space: &mut S) -> crate::AResult<AddressWidth> {
	// remember error from "safe" detection:
	let orig_err = match detect_address_width(space) {
		Ok(r) => return Ok(r),
		Err(e) => e,
	};

	// verify data actually starts with 0xff, so we know how to restore it
	space.ee_off()?;
	space.ee_sendbyte(READ_EE_OPCODE)?;
	for _ in 0..0x100 {
		if space.ee_readbyte()? != 0xff {
			// already found non-0xff data, but couldn't determine width from it
			// writing new non-0xff data won't help here
			return Err(orig_err);
		}
	}

	// try writing with increasing address width; if reading it back
	// fails it shouldn't have modified anything
	for aw in [AddressWidth::One, AddressWidth::Two, AddressWidth::Three].iter() {
		let mut flash = Flash {
			space: &mut space,
			address_width: *aw,
		};
		flash.write_byte(0, 0)?; // write data 0x00 at address 0
		// try to read it
		let data = flash.read_byte(0)?;
		if data != 0xff {
			// restore 0xff
			flash.write_byte(0, 0xff)?;
			// we really expect to read what we just wrote
			ensure!(data == 0x00, "address width detection failed: wrote 0x00 over 0xff, got 0x{:02x} back", data);
			return Ok(*aw);
		}
		// otherwise write shouldn't have succeeded, address width is
		// longer than `aw`
	}

	bail!("Couldn't detect address width even with writing 0x00 at address 0")
}

pub fn open_flash_recovery<S: PciConfigSpace>(mut space: S) -> crate::AResult<Flash<S>> {
	let endpoint = space.endpoint();

	// "PCI Base Address 0 Enable"
	// BAR0 is likely hardwired to be enabled, also we don't use it.
	// space.write_byte(0x48, 0x02);

	let eectl = space.eectl_read();
	if !eectl.is_present() {
		bail!("No EEPROM present");
	}
	if !eectl.is_valid() {
		warn!("PCI {}: EEPROM invalid", endpoint);
	}
	let address_width = match eectl.address_width() {
		None => {
			warn!("PCI {}: EEPROM address width unknown, trying to determine manually", endpoint);
			detect_address_width(&mut space)?
		},
		Some(aw) => aw,
	};

	let device_flags = space.main_read(DEVICE_INITIALIZATION);
	let pci_express_enabled = 0 != (device_flags & 0b1_0000);
	if !pci_express_enabled {
		warn!("PCI {}: PCI Express not enabled", endpoint);
	}
	let pci_enabled = 0 != (device_flags & 0b10_0000);
	if !pci_enabled {
		warn!("PCI {}: PCI not enabled", endpoint);
	}
	let frequency = device_flags & 0b1111;
	if frequency != 0b0011 {
		warn!("PCI {}: Speed not default (33.3/66/62.5)", endpoint);
	}

	Ok(Flash {
		space,
		address_width,
	})
}

pub fn is_pex8112_bridge(endpoint: PciEndpoint) -> crate::AResult<bool> {
	let vendor = endpoint.vendor()?;
	let device = endpoint.device()?;
	let class = endpoint.class()?;
	Ok(
		vendor.0 == 0x10b5 && device.0 == 0x8112
		&& class.class_code.0 == 0x06 // Bridge Device
		&& class.subclass_code.0 == 0x04 // PCI-to-PCI Bridge
		&& class.programming_interface.0 == 0x00 // Normal Decode
	)
}