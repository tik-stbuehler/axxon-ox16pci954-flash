/// Decode Local configuration registers

use std::io;

use crate::pci::{
	PciEndpoint,
	PciResourceReadOnly,
	open_resource_readonly,
};

pub mod local_configuration_types {
	#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
	pub enum Mode {
		UartAndEightBitLocalBus,
		UartAndParallelPort,
		UartAndSubsystemIDs,
		ThirtyTwoBitLocalBus,
	}

	// 8-bit local bus: which part of a dword
	#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
	pub enum EndianByteLane {
		Lane0, // 0..7
		Lane1, // 8..15
		Lane2, // 16..23
		Lane3, // 24..31
	}

	#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
	pub enum PowerDownFilterTime {
		Disabled,
		Wait4Seconds,
		Wait129Seconds,
		Wait518Seconds,
	}

	// Multi-purpose IO configuration
	#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
	pub enum MioConfiguration {
		NonInvertingInput,
		InvertingInput,
		OutputZero,
		OutputOne,
	}

	impl MioConfiguration {
		pub(super) fn from_bits(v: u8) -> Self {
			match v & 0x3 {
				0b00 => MioConfiguration::NonInvertingInput,
				0b01 => MioConfiguration::InvertingInput,
				0b10 => MioConfiguration::OutputZero,
				0b11 => MioConfiguration::OutputOne,
				_ => unreachable!(),
			}
		}
	}

	#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
	pub enum MioConfigurationOrPME {
		MioConfiguration(MioConfiguration),
		PME(bool),
	}
}

use self::local_configuration_types::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LocalConfiguration {
	mode: Mode,
	uart_clock_output: bool,
	endian_byte_lane: EndianByteLane,
	power_down_filter_time: PowerDownFilterTime,
	function1_mio2_pme_enable: bool,
	eeprom_data_in: bool,
	eeprom_valid: bool,
	eeprom_reload_in_progress: bool,
	mio0_config: Option<MioConfiguration>,
	mio1_config: Option<MioConfiguration>,
	mio2_config: MioConfigurationOrPME,
	mio3_config: MioConfiguration,
	mio4_config: MioConfiguration,
	mio5_config: MioConfiguration,
	mio6_config: MioConfiguration,
	mio7_config: MioConfiguration,
	mio8_config: MioConfiguration,
	mio9_config: MioConfiguration,
	mio10_config: MioConfiguration,
	mio11_config: MioConfiguration,
	local_bus_read_chip_select_assertion: u8, // 4-bit ?
	local_bus_read_chip_select_deassertion: u8, // 4-bit ?
	local_bus_write_chip_select_assertion: u8, // 4-bit ?
	local_bus_write_chip_select_deassertion: u8, // 4-bit ?
	local_bus_read_control_assertion: u8, // 4-bit ?
	local_bus_read_control_deassertion: u8, // 4-bit ?
	local_bus_write_control_assertion: u8, // 4-bit ?
	local_bus_write_control_deassertion: u8, // 4-bit ?
	local_bus_write_data_bus_control_assertion: u8, // 4-bit ?
	local_bus_write_data_bus_control_deassertion: u8, // 4-bit ?
	local_bus_read_data_bus_control_assertion: u8, // 4-bit ?
	local_bus_read_data_bus_control_deassertion: u8, // 4-bit ?
	function1_bar0_block_size: u8,
	local_bus_lower_address_cs_decode: u8,
	function1_bar1_block_size: Option<u8>,
	local_bus_software_reset: bool,
	local_bus_clock_enable: bool,
	local_bus_interface_type: bool,
	uart_receiver_levels: [u8; 4],
	uart_transmitter_levels: [u8; 4],
	uart_interrupt_source: [u8; 4],
	uart_good_status: [bool; 4],
	uart_global_good_status: bool,
	uart_interrupt_state: [bool; 4],
	mio_state: [bool; 12],
	uart_interrupt_mask: [bool; 4],
	mio_mask: [bool; 12],
}

pub fn decode_resource3(ep: PciEndpoint) -> io::Result<LocalConfiguration> {
	let mut buf = [0u8; 32];

	{
		let r3 = open_resource_readonly(ep, 3)?;
		if r3.len() != 4096 {
			return Err(io::Error::new(io::ErrorKind::Other, "unexpected resource length"));
		}
		// 4096 bytes reserved, but only 32 "real" bytes (other address bits are
		// ignored, so the data repeats itself)
		r3.read_slice(0, &mut buf[..]);
	}

	// byte 0:
	let mode = match buf[0x00] & 0x03 {
		0b00 => Mode::UartAndEightBitLocalBus,
		0b01 => Mode::UartAndParallelPort,
		0b10 => Mode::UartAndSubsystemIDs,
		0b11 => Mode::ThirtyTwoBitLocalBus,
		_ => unreachable!(),
	};

	let uart_clock_output = 0 != (buf[0x00] & 0x04);
	let endian_byte_lane = match (buf[0x00] >> 3) & 0x03 {
		0b00 => EndianByteLane::Lane0,
		0b01 => EndianByteLane::Lane1,
		0b10 => EndianByteLane::Lane2,
		0b11 => EndianByteLane::Lane3,
		_ => unreachable!(),
	};

	let power_down_filter_time = match (buf[0x00] >> 5) & 0x03 {
		0b00 => PowerDownFilterTime::Disabled,
		0b01 => PowerDownFilterTime::Wait4Seconds,
		0b10 => PowerDownFilterTime::Wait129Seconds,
		0b11 => PowerDownFilterTime::Wait518Seconds,
		_ => unreachable!(),
	};

	let function1_mio2_pme_enable = 0 != (buf[0x00] & 0x80);

	// ----------------------------------------------------
	// offset 0x00: LCC: Local Configuration and Control register

	// byte 0x01+0x02: reserved

	// byte 0x03: EEPROM

	let eeprom_data_in = 0 != (buf[0x03] & 0x08);

	let eeprom_valid = 0 != (buf[0x03] & 0x10);

	// probably never true
	let eeprom_reload_in_progress = 0 != (buf[0x03] & 0x20);

	// ----------------------------------------------------
	// offset 0x04: MIC: Multi-purpose I/O Configuration register

	// byte 0x04: Multi-purpose IO 0..3 configuration

	let mio0_config = match mode {
		Mode::UartAndParallelPort => None,
		_ => Some(MioConfiguration::from_bits(buf[0x04])),
	};

	let mio1_config = match power_down_filter_time {
		PowerDownFilterTime::Disabled => Some(MioConfiguration::from_bits(buf[0x04] >> 2)),
		_ => None,
	};

	let mio2_config = if function1_mio2_pme_enable {
		MioConfigurationOrPME::PME(0 != buf[0x04] & 0x10)
	} else {
		MioConfigurationOrPME::MioConfiguration(MioConfiguration::from_bits(buf[0x04] >> 4))
	};

	let mio3_config = MioConfiguration::from_bits(buf[0x04] >> 6);

	// byte 0x05: Multi-purpose IO 4..7 configuration

	let mio4_config = MioConfiguration::from_bits(buf[0x05]);
	let mio5_config = MioConfiguration::from_bits(buf[0x05] >> 2);
	let mio6_config = MioConfiguration::from_bits(buf[0x05] >> 4);
	let mio7_config = MioConfiguration::from_bits(buf[0x05] >> 6);

	// byte 0x06: Multi-purpose IO 8..11 configuration

	let mio8_config = MioConfiguration::from_bits(buf[0x06]);
	let mio9_config = MioConfiguration::from_bits(buf[0x06] >> 2);
	let mio10_config = MioConfiguration::from_bits(buf[0x06] >> 4);
	let mio11_config = MioConfiguration::from_bits(buf[0x06] >> 6);

	// byte 0x07: reserved

	// ----------------------------------------------------
	// offset 0x08: LT1: Local Bus Timing register 1

	// byte 0x08: Read Chip-select (De-)Assertion

	let local_bus_read_chip_select_assertion = buf[0x08] & 0x0f;
	let local_bus_read_chip_select_deassertion = buf[0x08] >> 4;

	// byte 0x09: Write Chip-select (De-)Assertion

	let local_bus_write_chip_select_assertion = buf[0x09] & 0x0f;
	let local_bus_write_chip_select_deassertion = buf[0x09] >> 4;

	// byte 0x0a: Read Control/Data-strobe (De-)Assertion

	let local_bus_read_control_assertion = buf[0x0a] & 0x0f;
	let local_bus_read_control_deassertion = buf[0x0a] >> 4;

	// byte 0x0b: Write Control/Data-strobe (De-)Assertion

	let local_bus_write_control_assertion = buf[0x0b] & 0x0f;
	let local_bus_write_control_deassertion = buf[0x0b] >> 4;

	// ----------------------------------------------------
	// offset 0x0c: LT2: Local Bus Timing register 2

	// byte 0x0c: Write Data Bus (De-)Assertion

	let local_bus_write_data_bus_control_assertion = buf[0x0c] & 0x0f;
	let local_bus_write_data_bus_control_deassertion = buf[0x0c] >> 4;

	// byte 0x0d: Read Data Bus (De-)Assertion

	let local_bus_read_data_bus_control_assertion = buf[0x0d] & 0x0f;
	let local_bus_read_data_bus_control_deassertion = buf[0x0d] >> 4;

	// byte 0x0e+0x0f: various

	let function1_bar0_block_size = match (buf[0x0e] >> 4) & 0x07 {
		// 0b000..0b111
		v => v
	};

	let local_bus_lower_address_cs_decode = match (buf[0x0e] >> 7) | ((buf[0x0f] & 0x07) << 1) {
		// 0b0000..0b1111
		v => v,
	};

	let function1_bar1_block_size = if mode == Mode::ThirtyTwoBitLocalBus {
		Some(match (buf[0x0f] >> 3) & 0x03 {
			// 0b00..0b11
			v => v,
		})
	} else {
		None
	};

	let local_bus_software_reset = 0 != (buf[0x0f] & 0x20);

	let local_bus_clock_enable = 0 != (buf[0x0f] & 0x40);

	// always false for parallel port mode
	let local_bus_interface_type = 0 != (buf[0x0f] & 0x80);

	// ----------------------------------------------------
	// offset 0x10: URL: UART Receiver FIFO Levels

	let uart_receiver_levels = [
		buf[0x10], buf[0x11], buf[0x12], buf[0x13],
	];

	// ----------------------------------------------------
	// offset 0x14: UTL: UART Transmitter FIFO Levels

	let uart_transmitter_levels = [
		buf[0x14], buf[0x15], buf[0x16], buf[0x17],
	];

	// ----------------------------------------------------
	// offset 0x18: UIS: UART Interrupt Source register

	let uart_interrupt_source = [
		buf[0x18] & 0x3f,
		((buf[0x19] & 0xf) << 2) | ((buf[0x18] >> 6)),
		((buf[0x1a] & 0x3) << 4) | ((buf[0x19] >> 4)),
		buf[0x1a] >> 2,
	];

	let uart_good_status = [
		0 != (buf[0x1b] & 0x08),
		0 != (buf[0x1b] & 0x10),
		0 != (buf[0x1b] & 0x20),
		0 != (buf[0x1b] & 0x40),
	];
	let uart_global_good_status = 0 != (buf[0x1b] & 0x80);

	// ----------------------------------------------------
	// offset 0x1c: GIS: Global Interrupt Status and control register

	let uart_interrupt_state = [
		0 != (buf[0x1c] & 0x01),
		0 != (buf[0x1c] & 0x02),
		0 != (buf[0x1c] & 0x04),
		0 != (buf[0x1c] & 0x08),
	];

	let mio_state = [
		0 != (buf[0x1c] & 0x10),
		0 != (buf[0x1c] & 0x20),
		0 != (buf[0x1c] & 0x40),
		0 != (buf[0x1c] & 0x80),
		0 != (buf[0x1d] & 0x01),
		0 != (buf[0x1d] & 0x02),
		0 != (buf[0x1d] & 0x04),
		0 != (buf[0x1d] & 0x08),
		0 != (buf[0x1d] & 0x10),
		0 != (buf[0x1d] & 0x20),
		0 != (buf[0x1d] & 0x40),
		0 != (buf[0x1d] & 0x80),
	];

	let uart_interrupt_mask = [
		0 != (buf[0x1e] & 0x01),
		0 != (buf[0x1e] & 0x02),
		0 != (buf[0x1e] & 0x04),
		0 != (buf[0x1e] & 0x08),
	];

	let mio_mask = [
		0 != (buf[0x1e] & 0x10),
		0 != (buf[0x1e] & 0x20),
		0 != (buf[0x1e] & 0x40),
		0 != (buf[0x1e] & 0x80),
		0 != (buf[0x1f] & 0x01),
		0 != (buf[0x1f] & 0x02),
		0 != (buf[0x1f] & 0x04),
		0 != (buf[0x1f] & 0x08),
		0 != (buf[0x1f] & 0x10),
		0 != (buf[0x1f] & 0x20),
		0 != (buf[0x1f] & 0x40),
		0 != (buf[0x1f] & 0x80),
	];

	Ok(LocalConfiguration{
		mode,
		uart_clock_output,
		endian_byte_lane,
		power_down_filter_time,
		function1_mio2_pme_enable,
		eeprom_data_in,
		eeprom_valid,
		eeprom_reload_in_progress,
		mio0_config,
		mio1_config,
		mio2_config,
		mio3_config,
		mio4_config,
		mio5_config,
		mio6_config,
		mio7_config,
		mio8_config,
		mio9_config,
		mio10_config,
		mio11_config,
		local_bus_read_chip_select_assertion,
		local_bus_read_chip_select_deassertion,
		local_bus_write_chip_select_assertion,
		local_bus_write_chip_select_deassertion,
		local_bus_read_control_assertion,
		local_bus_read_control_deassertion,
		local_bus_write_control_assertion,
		local_bus_write_control_deassertion,
		local_bus_write_data_bus_control_assertion,
		local_bus_write_data_bus_control_deassertion,
		local_bus_read_data_bus_control_assertion,
		local_bus_read_data_bus_control_deassertion,
		function1_bar0_block_size,
		local_bus_lower_address_cs_decode,
		function1_bar1_block_size,
		local_bus_software_reset,
		local_bus_clock_enable,
		local_bus_interface_type,
		uart_receiver_levels,
		uart_transmitter_levels,
		uart_interrupt_source,
		uart_good_status,
		uart_global_good_status,
		uart_interrupt_state,
		mio_state,
		uart_interrupt_mask,
		mio_mask,
	})
}
