mod decode;
mod eeprom;

pub use self::decode::{
	LocalConfiguration,
	decode_resource3,
	local_configuration_types,
};

pub use self::eeprom::open_eeprom;

use crate::pci::PciEndpoint;
use crate::serial::HardwareOperations;

pub fn is_ox16_pci954(ep: PciEndpoint) -> crate::AResult<bool> {
	match (ep.vendor()?.0, ep.device()?.0) {
		(0x1415, 0x9500) => Ok(true), // function 0: disabled
		(0x1415, 0x9501) => Ok(true), // function 0: Uart
		(0x1415, 0x9510) => Ok(true), // function 1: disabled
		(0x1415, 0x9511) => Ok(true), // function 1: 8-bit bus
		(0x1415, 0x9512) => Ok(true), // function 1: 32-bit bus
		(0x1415, 0x9513) => Ok(true), // function 1: parallel port
		_ => Ok(false),
	}
}


/// From flash tool "LF729KB" with compile date: 06-29-2016
// zone0 (header):
// - 0x9505: magic 0x950*, zone1 (flag 0x4) and zone3 (flag 0x1) present
// zone1: (flag 0x8000: read another word in zone1, bits 0x7f00 (>> 8): register to write, bits 0xff: value to write)
// - 0x84ff: 0x04 -> 0xff: Multi-purpose IO configuration: MIC[7:0] = 0b1111_1111
// - 0x85ff: 0x05 -> 0xff: Multi-purpose IO configuration: MIC[15:8] = 0b1111_1111
// - 0x86ff: 0x06 -> 0xff: Multi-purpose IO configuration: MIC[23:16] = 0b1111_1111
// - 0x9e0f: 0x1e -> 0x0f:
//   - UART Interrupt Mask GIS[19:16] = 0b1111 (default)
//   - MIO0 / Parallel Port Interrupt Mask GIS[20] = 0b0
//   - Multi-purpose IO Interrupt Mask GIS[23:21] = 0b000
// - 0x1f00: 0x1f -> 0x00:
//   - Multi-purpose IO Interrupt Mask GIS[31:24] = 0b0000_0000
// zone3:
// - 0x8001: configure function1
//   - 0x8200: 0x02 -> 0x00: Device ID low byte: 0x00
//   - 0x3d00: 0x3d -> 0x00: Interrupt pin: 0
// - 0x0000: end of zone3
pub const IMAGE: [u16; 10] = [
	0x9505, 0x84ff, 0x85ff, 0x86ff,
	0x9e0f, 0x1f00, 0x8001, 0x8200,
	0x3d00, 0x0000,
];

pub fn flash_program<H>(hardware: &mut H, program: &[u16]) -> crate::AResult<()>
where
	H: HardwareOperations,
{
	{
		let mut hw_prog = hardware.start_programming()?;
		hw_prog.erase_all()?;
		for address in 0..program.len() {
			hw_prog.write(address, program[address])?;
		}
	}
	for address in 0..program.len() {
		let flash = hardware.read(address)?;
		ensure!(flash == program[address],
			"Verify failed at {:02x}: expected {:04x}, flash is {:04x}", address, program[address], flash
		);
	}

	Ok(())
}

pub fn read_flash_program<H>(hardware: &mut H) -> crate::AResult<Vec<u16>>
where
	H: HardwareOperations,
{
	let mut buf = Vec::new();
	let mut reader = hardware.read_all()?;
	buf.push(reader.next().ok_or_else(|| format_err!("Unexpected end of flash data"))?);
	if buf[0] == 0xffff {
		warn!("Flash empty");
		return Ok(Vec::new());
	}
	ensure!(buf[0] & 0xfff0 == 0x9500, "Invalid magic: 0x{:04x} (expected 0x9500)", buf[0] & 0xfff0);
	ensure!(buf[0] & 0x0008 == 0, "Invalid zone flags: 0x{:04x} (should be zero)", buf[0] & 0x0008);
	let zone1 = buf[0] & 0x0004 != 0;
	let zone2 = buf[0] & 0x0002 != 0;
	let zone3 = buf[0] & 0x0001 != 0;

	if zone1 {
		loop {
			let w = reader.next().ok_or_else(|| format_err!("Unexpected end of flash data"))?;
			buf.push(w);
			if w & 0x8000 == 0 { break; }
		}
	}
	if zone2 {
		loop {
			let w = reader.next().ok_or_else(|| format_err!("Unexpected end of flash data"))?;
			buf.push(w);
			if w & 0x8000 == 0 { break; }
		}
	}
	if zone3 {
		loop {
			let w = reader.next().ok_or_else(|| format_err!("Unexpected end of flash data"))?;
			buf.push(w);
			if w & 0x8000 == 0 { break; }
			loop {
				let w = reader.next().ok_or_else(|| format_err!("Unexpected end of flash data"))?;
				buf.push(w);
				if w & 0x8000 == 0 { break; }
			}
		}
	}

	Ok(buf)
}
