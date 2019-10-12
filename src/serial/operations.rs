use super::{
	Hardware,
	LowLevel,
	low_level::ReadTransaction,
};

const ADDRESS_WIDTH: usize = 6;
const ADDRESS_LIMIT: usize = (1usize << ADDRESS_WIDTH);

pub struct Reader<'a, H: Hardware + ?Sized + 'a> {
	remaining: usize,
	transaction: ReadTransaction<'a, H>,
}

impl<'a, H: Hardware + ?Sized> Iterator for Reader<'a, H> {
	type Item = u16;

	fn next(&mut self) -> Option<Self::Item> {
		if 0 == self.remaining {
			return None;
		}
		self.remaining -= 1;
		Some(self.transaction.receive_word())
	}
}

mod inner {
	use super::*;

	pub trait HardwareOperationsBase {
		type Hardware: Hardware + ?Sized;

		fn hardware(&mut self) -> &mut Self::Hardware;

		fn read_unknown_address_width(&mut self) -> crate::AResult<(ReadTransaction<Self::Hardware>, usize)> {
			let mut tx = self.hardware().start_transaction();

			tx.send_bit(true)?;
			tx.send_bit(false)?;

			let (mut tx, first_bit) = tx.force_receive();

			let mut len = 0usize;

			if first_bit {
				len += 1;
				while tx.receive_bit() {
					len += 1;
					ensure!(len <= 16, "only detecting address width up to 16 bits allowed to prevent endless loop");
				}
			}

			Ok((tx, len))
		}
	}

	impl<H: Hardware + ?Sized> HardwareOperationsBase for H {
		type Hardware = H;


		fn hardware(&mut self) -> &mut Self::Hardware {
			self
		}
	}
}

pub trait HardwareOperations: inner::HardwareOperationsBase {
	fn erase(&mut self, address: usize) -> crate::AResult<()> {
		assert!(address < ADDRESS_LIMIT);
		let mut tx = self.hardware().start_program_transaction();
		tx.send_bit(true)?;
		tx.send_bit(true)?;
		tx.send_bits(address as u16, ADDRESS_WIDTH)
	}

	fn erase_all(&mut self) -> crate::AResult<()> {
		let mut tx = self.hardware().start_program_transaction();
		tx.send_bits(0b00_10_0000, 8)
	}

	fn erase_write_disable(&mut self) -> crate::AResult<()> {
		let mut tx = self.hardware().start_program_transaction();
		tx.send_bits(0b00_00_0000, 8)
	}

	fn erase_write_enable(&mut self) -> crate::AResult<()> {
		let mut tx = self.hardware().start_program_transaction();
		tx.send_bits(0b00_11_0000, 8)
	}

	fn detect_address_width(&mut self) -> crate::AResult<usize> {
		let (_, len) = self.read_unknown_address_width()?;
		Ok(len)
	}

	fn read(&mut self, address: usize) -> crate::AResult<u16> {
		assert!(address < ADDRESS_LIMIT);
		let mut tx = self.hardware().start_transaction();

		tx.send_bit(true)?;
		tx.send_bit(false)?;
		tx.send_bits(address as u16, ADDRESS_WIDTH)?;
		let mut tx = tx.start_receive()?;
		let result = tx.receive_word();

		Ok(result)
	}

	fn read_all(&mut self) -> crate::AResult<Reader<Self::Hardware>> {
		let (transaction, len) = self.read_unknown_address_width()?;

		Ok(Reader {
			remaining: 1 << len,
			transaction,
		})
	}

	fn write(&mut self, address: usize, word: u16) -> crate::AResult<()> {
		assert!(address < ADDRESS_LIMIT);
		let mut tx = self.hardware().start_program_transaction();
		tx.send_bit(false)?;
		tx.send_bit(true)?;
		tx.send_bits(address as u16, ADDRESS_WIDTH)?;
		tx.send_bits(word, 16)
	}

	// write one word into all addresses; includes erasing beforre
	fn write_all(&mut self, word: u16) -> crate::AResult<()> {
		let mut tx = self.hardware().start_program_transaction();
		tx.send_bits(0b00_01_0000, 8)?;
		tx.send_bits(word, 16)
	}

	fn start_programming(&mut self) -> crate::AResult<ProgrammingEnabled<Self>>;
}

impl<H: ?Sized+Hardware> HardwareOperations for H {
	fn start_programming(&mut self) -> crate::AResult<ProgrammingEnabled<Self>> {
		self.erase_write_enable()?;
		Ok(ProgrammingEnabled(self, true))
	}
}

pub struct ProgrammingEnabled<'a, H: ?Sized+HardwareOperations+'a>(&'a mut H, bool);

impl<'a, H: ?Sized+HardwareOperations> Drop for ProgrammingEnabled<'a, H> {
	fn drop(&mut self) {
		if self.1 {
			if let Err(e) = self.0.erase_write_disable() {
				eprintln!("Couldn't disable Erase/Write mode: {}", e);
			}
		}
	}
}

impl<'a, H: ?Sized+HardwareOperations> inner::HardwareOperationsBase for ProgrammingEnabled<'a, H> {
	type Hardware = H::Hardware;

	fn hardware(&mut self) -> &mut Self::Hardware {
		self.0.hardware()
	}
}

impl<'a, H: ?Sized+HardwareOperations> HardwareOperations for ProgrammingEnabled<'a, H> {
	fn start_programming(&mut self) -> crate::AResult<ProgrammingEnabled<Self>> {
		Ok(ProgrammingEnabled(self, false))
	}
}
