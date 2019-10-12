use std::ops::{
	Deref,
	DerefMut,
};

use super::{
	Hardware,
	OutPins,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Signal {
	Clear,
	Zero,
	One,
}

impl Signal {
	pub fn with_clock(self, clock: bool) -> OutPins {
		let (chip_select, data) = match self {
			Signal::Clear => (false, false),
			Signal::Zero => (true, false),
			Signal::One => (true, true),
		};
		OutPins {
			chip_select,
			clock,
			data,
		}
	}
}

impl From<bool> for Signal {
	fn from(v: bool) -> Self {
		match v {
			false => Signal::Zero,
			true => Signal::One,
		}
	}
}

pub struct Transaction<'a, H: ?Sized+LowLevel+'a>(&'a mut H);

impl<'a, H: ?Sized+LowLevel> Transaction<'a, H> {
	// for receiving we're gonna cycle CLK the other way
	//
	// this will wait for half a cycle while CLK is low
	//
	// CS needs to be up all the time, and DATA down.
	pub fn start_receive(self) -> crate::AResult<ReadTransaction<'a, H>> {
		let (tx, data) = self.force_receive();
		ensure!(!data, "receiving needs to be prefixed by 0 bit");
		Ok(tx)
	}

	// force receive mode, need to search for 0-bit prefix manually; also return bit from end of current cycle
	pub(super) fn force_receive(mut self) -> (ReadTransaction<'a, H>, bool) {
		// move CLK phase
		self.set_pins(Signal::Zero.with_clock(false));
		self.delay();
		let data = self.read_pin();

		(ReadTransaction(self), data)
	}
}

impl<'a, H: ?Sized+LowLevel> Drop for Transaction<'a, H> {
	fn drop(&mut self) {
		self.0._finish_instruction();
	}
}

impl<'a, H: ?Sized+LowLevel> Deref for Transaction<'a, H> {
	type Target = H;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'a, H: ?Sized+LowLevel> DerefMut for Transaction<'a, H> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

pub struct ReadTransaction<'a, H: ?Sized+LowLevel+'a>(Transaction<'a, H>);

impl<'a, H: ?Sized+LowLevel> ReadTransaction<'a, H> {
	// drive CLK up and down; read input after a full CLK cycle passed after
	// positive CLK edge.
	pub(super) fn receive_bit(&mut self) -> bool {
		self.set_pins(Signal::Zero.with_clock(true));
		self.delay();
		self.set_pins(Signal::Zero.with_clock(false));
		self.delay();
		self.read_pin()
	}

	// read 16-bit word, starting with highest bit
	pub fn receive_word(&mut self) -> u16 {
		let mut result = 0u16;
		for bit in (0..16).rev() {
			let bit_mask = 1u16 << bit;
			// send zero bit for each bit we want to read
			if self.receive_bit() {
				result |= bit_mask;
			}
		}
		result
	}
}

impl<'a, H: ?Sized+LowLevel> Deref for ReadTransaction<'a, H> {
	type Target = H;

	fn deref(&self) -> &Self::Target {
		&(self.0).0
	}
}

impl<'a, H: ?Sized+LowLevel> DerefMut for ReadTransaction<'a, H> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut (self.0).0
	}
}

pub struct ProgramTransaction<'a, H: ?Sized+LowLevel+'a>(&'a mut H);

impl<'a, H: ?Sized+LowLevel> Drop for ProgramTransaction<'a, H> {
	fn drop(&mut self) {
		self.0._wait_for_completion();
	}
}

impl<'a, H: ?Sized+LowLevel> Deref for ProgramTransaction<'a, H> {
	type Target = H;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'a, H: ?Sized+LowLevel> DerefMut for ProgramTransaction<'a, H> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

trait InternalLowLevel: Hardware {
	// prepare chip_select/data during CLK lo, then bring CLK up
	//
	// waits for the full CLK-HIGH edge after pulling it up
	fn signal(&mut self, signal: Signal) {
		let no_clk = signal.with_clock(false);
		let clk = signal.with_clock(true);

		self.set_pins(no_clk);
		self.delay(); // wait for pins to be stable

		self.set_pins(clk);
		self.delay(); // wait for chip reading the pins
	}

	// similar to `signal`, but also reads input before dropping CLK
	fn signal_and_read(&mut self, signal: Signal) -> bool {
		let no_clk = signal.with_clock(false);
		let clk = signal.with_clock(true);

		self.set_pins(no_clk);
		self.delay(); // wait for pins to be stable

		self.set_pins(clk);
		self.delay(); // wait for chip reading the pins
		let result = self.read_pin();

		self.set_pins(no_clk);
		// no need to wait here: data is read on the rising CLK

		result
	}

	// start instruction
	fn _start_instruction(&mut self) {
		// make sure chip isn't BUSY
		self._wait_for_completion();

		// now trigger CLK up and down while CS + DATA is up, in short: a "1 bit"
		self.signal(Signal::One);
	}

	// turn all pins off and wait for a half cycle
	fn _finish_instruction(&mut self) {
		self.set_pins(Signal::Clear.with_clock(false));
		self.delay();
	}

	// wait for previous write/erase instruction to finish; also clears at the end
	fn _wait_for_completion(&mut self) {
		// one cycle with low CS + DATA
		self.signal(Signal::Clear);
		// now send 0 bits until data input is high; data input should be pulled
		// up by default. chip will pull data down when it gets enabled and it
		// still is BUSY, signal should be ready after the clock cycle.
		//
		// if chip wasn't BUSY it should have "don't care" on data, i.e. it
		// should read as "1"
		//
		// when chip becomes READY after BUSY it will pull up the pin too.
		self.signal(Signal::Zero);
		// the read input should be stable until we drop CS, independent of CLK
		//
		// Timing: "status valid" becomes ready after a full CLK cycle with CS,
		// which we just did
		while !self.read_pin() {
			// technicall we could just wait and poll, but let's drive CLK too.
			self.signal(Signal::Zero);
		}
		self._finish_instruction();
	}
}

impl<H: Hardware+?Sized> InternalLowLevel for H {
}

pub trait LowLevel: Hardware {
	// similar to `signal`, but also reads input before dropping CLK
	//
	// Data output delay time is 400ns; so we read data after 500 ns (after a full
	// CLK cycle before the next CLK positive edge)
	fn send_bit_and_read_previous(&mut self, data: bool) -> bool {
		let signal = Signal::from(data);
		let no_clk = signal.with_clock(false);
		let clk = signal.with_clock(true);

		self.set_pins(no_clk);
		self.delay(); // wait for pins to be stable

		// read previous bit before rising CLK
		let result = self.read_pin();

		self.set_pins(clk);
		self.delay(); // wait for chip reading the pins

		result
	}

	// similar to `send_bit`, but make sure we didn't recv data in previous cycle
	// (i.e. input stayed HIGH)
	fn send_bit(&mut self, data: bool) -> crate::AResult<()> {
		ensure!(self.send_bit_and_read_previous(data), "unexpected LOW input");
		Ok(())
	}

	// send `num` lowest bits from word, starting with highest bit; checks that
	// data from previos cycle is HIGH
	fn send_bits(&mut self, word: u16, num: usize) -> crate::AResult<()> {
		assert!(num <= 16);
		for bit in (0..num).rev() {
			let bit_mask = 1u16 << bit;
			self.send_bit(0 != (word & bit_mask))?;
		}

		Ok(())
	}

	fn start_transaction(&mut self) -> Transaction<Self> {
		self._start_instruction();

		Transaction(self)
	}

	fn start_program_transaction(&mut self) -> ProgramTransaction<Self> {
		self._start_instruction();

		ProgramTransaction(self)
	}
}

impl<H: Hardware+?Sized> LowLevel for H {
}
