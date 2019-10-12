/// Protocol for Microchip 93C46B, a 1-kbit EEPROM (organized as 64 x 16bit)
///
/// Sometimes called "I²C", but it really isn't. For example there are separate
/// pins for data IN and OUT, and there is also a CHIP SELECT pin.
///
/// Instructions:
/// - Startbit: "1"
/// - 2-bit Opcode
/// - 6-bit Address
///
/// Some instructions have a DATA phase following (either send or recv) for 16
/// bits, so the total request takes either 9 or 16 CLK cycles.
///
/// Opcodes: (@ address)
/// - 0b11: ERASE at address (set all bits to "1")
/// - 0b00 @ 0b00????: EWDS (erase/write disable), no DATA
/// - 0b00 @ 0b01????: WRAL (write all), DATA (for what?)
/// - 0b00 @ 0b10????: ERAL (erase all), no DATA
/// - 0b00 @ 0b11????: EWEN (erase/write enable), no DATA
/// - 0b10: READ 16-bits from address, recv DATA
/// - 0b01: WRITE 16-bits to address, send DATA

mod hardware;
mod low_level;
mod operations;

pub use self::hardware::{
	Hardware,
	OutPins,
};

use self::low_level::LowLevel;

pub use self::operations::{
	HardwareOperations,
};
