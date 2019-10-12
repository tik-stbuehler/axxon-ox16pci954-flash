#![allow(dead_code)]
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressWidth {
	One,
	Two,
	Three,
}

// EEPROM control flags
const EEPROM_WRITE_DATA_MASK:    u32 = 0x0000_00ff;
const EEPROM_READ_DATA_MASK:     u32 = 0x0000_ff00; // read only
const EEPROM_BYTE_WRITE_START:   u32 = 0x0001_0000;
const EEPROM_BYTE_READ_START:    u32 = 0x0002_0000;
const EEPROM_CHIP_SELECT:        u32 = 0x0004_0000;
const EEPROM_BUSY:               u32 = 0x0008_0000; // read only
const EEPROM_VALID:              u32 = 0x0010_0000; // read only
const EEPROM_PRESENT:            u32 = 0x0020_0000; // read only
const EEPROM_CHIP_SELECT_ACTIVE: u32 = 0x0040_0000; // read only
const EEPROM_ADDRESS_WIDTH_SHIFT: u8 = 23; // read only
const EEPROM_ADDRESS_WIDTH_MASK: u32 = 0x0180_0000; // read only
const EEPROM_RELOAD:             u32 = 0x8000_0000;

const SAFE_WRITE_FLAGS: u32 = 0
	| EEPROM_WRITE_DATA_MASK
	| EEPROM_BYTE_WRITE_START
	| EEPROM_BYTE_READ_START
	| EEPROM_CHIP_SELECT
;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EeControlRead(pub u32);

impl EeControlRead {
	// keep flags/bits that are safe to write back
	//
	// after `ee_waitidle` byte_write_start and byte_read_start should
	// be zero, and the data written last time is not very interesting,
	// and you should know whether you want CHIP_SELECT or not in the
	// next write anyway, so constructing a fresh `EeControlWrite` is
	// probably better anyway.
	pub fn into_write(&self) -> EeControlWrite {
		EeControlWrite(self.0 & SAFE_WRITE_FLAGS)
	}

	pub fn get_write_data(&self) -> u8 {
		self.0 as u8
	}

	// read data
	pub fn data(&self) -> u8 {
		(self.0 >> 8) as u8
	}

	pub fn address_width(&self) -> Option<AddressWidth> {
		match (self.0 & EEPROM_ADDRESS_WIDTH_MASK) >> EEPROM_ADDRESS_WIDTH_SHIFT {
			0x00 => None,
			0x01 => Some(AddressWidth::One),
			0x02 => Some(AddressWidth::Two),
			0x03 => Some(AddressWidth::Three),
			_ => unreachable!(),
		}
	}

	pub fn is_byte_write_start(&self) -> bool {
		0 != self.0 & EEPROM_BYTE_WRITE_START
	}
	pub fn is_byte_read_start(&self) -> bool {
		0 != self.0 & EEPROM_BYTE_READ_START
	}
	pub fn is_chip_select(&self) -> bool {
		0 != self.0 & EEPROM_CHIP_SELECT
	}
	pub fn is_busy(&self) -> bool {
		0 != self.0 & EEPROM_BUSY
	}
	pub fn is_valid(&self) -> bool {
		0 != self.0 & EEPROM_VALID
	}
	pub fn is_present(&self) -> bool {
		0 != self.0 & EEPROM_PRESENT
	}
	pub fn is_chip_select_active(&self) -> bool {
		0 != self.0 & EEPROM_CHIP_SELECT_ACTIVE
	}
	pub fn is_initialized(&self) -> bool {
		0 != self.0 & EEPROM_RELOAD
	}

	pub fn is_off(&self) -> bool {
		const OFF_MASK: u32 = 0
			| EEPROM_WRITE_DATA_MASK
			| EEPROM_BYTE_WRITE_START
			| EEPROM_BYTE_READ_START
			| EEPROM_CHIP_SELECT
			// EEPROM_RELOAD should be 1 when reading, but 0 when writing
		;

		0 == self.0 & OFF_MASK
	}

}

impl fmt::Display for EeControlRead {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "0x{:08x}", self.0)
	}
}

impl fmt::Debug for EeControlRead {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,
			"0x{:08x} (write: 0x{:02x}, read: 0x{:02x}, address width: {:?}",
			self.0,
			self.get_write_data(),
			self.data(),
			self.address_width(),
		)?;
		if self.is_byte_write_start() { write!(f, " [WR]")?; }
		if self.is_byte_read_start() { write!(f, " [RD]")?; }
		if self.is_chip_select() { write!(f, " [CS]")?; }
		if self.is_busy() { write!(f, " [BUSY]")?; }
		if self.is_valid() { write!(f, " [VALID]")?; }
		if self.is_present() { write!(f, " [PRESENT]")?; }
		if self.is_chip_select_active() { write!(f, " [CSA]")?; }
		if self.is_initialized() { write!(f, " [INITIALIZED]")?; }
		write!(f, ")")
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EeControlWrite(pub u32);

impl EeControlWrite {
	pub fn off() -> Self {
		EeControlWrite(0)
	}

	pub fn write_data(data: u8) -> Self {
		*EeControlWrite(0)
			.set_data(data)
			.set_byte_write_start()
			.set_chip_select()
	}

	pub fn read_data() -> Self {
		*EeControlWrite(0)
			.set_byte_read_start()
			.set_chip_select()
	}

	pub fn data(&self) -> u8 {
		self.0 as u8
	}

	pub fn set_data(&mut self, data: u8) -> &mut Self {
		self.0 = (self.0 & !0xff) | (data as u32);
		self
	}

	pub fn is_byte_write_start(&self) -> bool {
		0 != self.0 & EEPROM_BYTE_WRITE_START
	}
	pub fn set_byte_write_start(&mut self) -> &mut Self {
		self.0 = self.0 | EEPROM_BYTE_WRITE_START;
		self
	}
	pub fn clear_byte_write_start(&mut self) -> &mut Self {
		self.0 = self.0 & !EEPROM_BYTE_WRITE_START;
		self
	}

	pub fn is_byte_read_start(&self) -> bool {
		0 != self.0 & EEPROM_BYTE_READ_START
	}
	pub fn set_byte_read_start(&mut self) -> &mut Self {
		self.0 = self.0 | EEPROM_BYTE_READ_START;
		self
	}
	pub fn clear_byte_read_start(&mut self) -> &mut Self {
		self.0 = self.0 & !EEPROM_BYTE_READ_START;
		self
	}

	pub fn is_chip_select(&self) -> bool {
		0 != self.0 & EEPROM_CHIP_SELECT
	}
	pub fn set_chip_select(&mut self) -> &mut Self {
		self.0 = self.0 | EEPROM_CHIP_SELECT;
		self
	}
	pub fn clear_chip_select(&mut self) -> &mut Self {
		self.0 = self.0 & !EEPROM_CHIP_SELECT;
		self
	}

	pub fn is_reload(&self) -> bool {
		0 != self.0 & EEPROM_RELOAD
	}
	pub fn set_reload(&mut self) -> &mut Self {
		self.0 = self.0 | EEPROM_RELOAD;
		self
	}
	pub fn clear_reload(&mut self) -> &mut Self {
		self.0 = self.0 & !EEPROM_RELOAD;
		self
	}
}

impl fmt::Display for EeControlWrite {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "0x{:08x}", self.0)
	}
}

impl fmt::Debug for EeControlWrite {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,
			"0x{:08x} (write: 0x{:02x}",
			self.0,
			self.data(),
		)?;
		if self.is_byte_write_start() { write!(f, " [WR]")?; }
		if self.is_byte_read_start() { write!(f, " [RD]")?; }
		if self.is_chip_select() { write!(f, " [CS]")?; }
		if self.is_reload() { write!(f, " [RELOAD]")?; }
		write!(f, ")")
	}
}
