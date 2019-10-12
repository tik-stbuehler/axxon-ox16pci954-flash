use std::fmt;
use std::fs;
use std::io::{
	self,
	Read,
	Write,
};
use std::num::ParseIntError;
use std::str;

use super::Driver;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotFunction(pub u8);

impl SlotFunction {
	pub fn slot(&self) -> u8 {
		self.0 >> 3
	}

	pub fn function(&self) -> u8 {
		self.0 & 0x7
	}
}

impl fmt::Debug for SlotFunction {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("SlotFunction")
			.field("slot", &(self.0 >> 3))
			.field("function", &(self.0 & 0x7))
			.finish()
	}
}

impl fmt::Display for SlotFunction {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:02x}.{}", self.0 >> 3, self.0 & 0x7)
	}
}

impl str::FromStr for SlotFunction {
	type Err = ::failure::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let r = s.as_bytes();

		ensure!(r.len() <= 4, "String too long for PCI device.function: {:?}", s);

		// short: 0.0, long: 1f.7
		let (dev_s, fun_s) = if r.len() == 3 && r[1] == b'.' {
			(&s[0..1], &s[2..3])
		} else if r.len() == 4 && r[2] == b'.' {
			(&s[0..2], &s[3..4])
		} else {
			bail!("Couldn't find '.' in valid place for PCI device.function: {:?}", s);
		};

		let dev = with_context!(("invalid PCI device: {}", dev_s),
			u8::from_str_radix(dev_s, 16).map_err(|e| e.into())
		)?;
		let fun = with_context!(("invalid PCI function: {}", fun_s),
			Ok(u8::from_str_radix(fun_s, 8)?)
		)?;

		ensure!(dev < 0x20, "invalid PCI device: {} (too big)", dev);
		ensure!(fun <= 0x08, "invalid PCI function: {} (too big)", fun);

		Ok(SlotFunction(dev << 3 | fun))
	}
}

fn read_trimmed_info_file(ep: PciEndpoint, name: &str) -> crate::AResult<String> {
	with_context!(("couldn't read info file {} for PCI device {}", name, ep), {
		let mut f = fs::File::open(ep.device_file(name))?;
		let mut result = String::new();
		f.read_to_string(&mut result)?;
		Ok(result.trim().into())
	})
}

fn read_hex_info_file<T>(ep: PciEndpoint, name: &str, from_str_radix: fn(&str, u32) -> Result<T, ParseIntError>) -> crate::AResult<T> {
	let value = read_trimmed_info_file(ep, name)?;
	ensure!(value.starts_with("0x"), "info {} for PCI device {} doesn't start with '0x': {:?}", name, ep, value);
	with_context!(("couldn't parse info {} for PCI device {}", name, ep), {
		Ok(from_str_radix(&value[2..], 16)?)
	})
}

fn read_decimal_info_file<T>(ep: PciEndpoint, name: &str, from_str_radix: fn(&str, u32) -> Result<T, ParseIntError>) -> crate::AResult<T> {
	let value = read_trimmed_info_file(ep, name)?;
	with_context!(("couldn't parse info {} for PCI device {}", name, ep), {
		Ok(from_str_radix(&value, 10)?)
	})
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PciBus {
	pub domain: u16,
	pub bus: u8,
}

impl fmt::Display for PciBus {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:04x}:{:02x}", self.domain, self.bus)
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PciEndpoint {
	pub bus: PciBus,
	pub slot_function: SlotFunction,
}

impl PciEndpoint {
	fn device_file(&self, name: &str) -> String {
		format!("/sys/bus/pci/devices/{}/{}", *self, name)
	}

	pub fn is_enabled(&self) -> crate::AResult<bool> {
		match read_trimmed_info_file(*self, "enable")?.as_str() {
			"0" => Ok(false),
			"1" => Ok(true),
			e => bail!("Invalid 'enable' value {:?} for PCI device {}", e, self),
		}
	}

	pub fn scoped_enable(&self) -> crate::AResult<ScopedEnable> {
		if !self.is_enabled()? {
			let scoped_enable = ScopedEnable { ep: Some(*self) };
			self.enable()?;
			Ok(scoped_enable)
		} else {
			Ok(ScopedEnable { ep: None })
		}
	}

	pub fn enable(&self) -> crate::AResult<()> {
		with_context!(("PCI {}: enable device", self), {
			fs::OpenOptions::new().write(true).open(self.device_file("enable"))?.write_all(b"1")?;
			Ok(())
		})
	}

	pub fn disable(&self) -> crate::AResult<()> {
		with_context!(("PCI {}: disable device", self), {
			fs::OpenOptions::new().write(true).open(self.device_file("enable"))?.write_all(b"0")?;
			Ok(())
		})
	}

	pub fn vendor(&self) -> crate::AResult<VendorId> {
		read_hex_info_file::<u16>(*self, "vendor", u16::from_str_radix).map(VendorId)
	}

	pub fn device(&self) -> crate::AResult<DeviceID> {
		read_hex_info_file::<u16>(*self, "device", u16::from_str_radix).map(DeviceID)
	}

	pub fn subsystem_vendor(&self) -> crate::AResult<VendorId> {
		read_hex_info_file::<u16>(*self, "subsystem_vendor", u16::from_str_radix).map(VendorId)
	}

	pub fn subsystem_device(&self) -> crate::AResult<DeviceID> {
		read_hex_info_file::<u16>(*self, "subsystem_device", u16::from_str_radix).map(DeviceID)
	}

	pub fn class(&self) -> crate::AResult<Class> {
		let v = read_hex_info_file::<u32>(*self, "class", u32::from_str_radix)?;
		let class_code = ClassCode((v >> 16) as u8);
		let subclass_code = SubClassCode((v >> 8) as u8);
		let programming_interface = ProgrammingInterface(v as u8);
		Ok(Class{class_code, subclass_code, programming_interface})
	}

	/// Bridges have a secondary bus (the bus directly connected devices on the other side are on)
	pub fn secondary_bus(&self) -> crate::AResult<PciBus> {
		let bus = read_decimal_info_file::<u8>(*self, "secondary_bus_number", u8::from_str_radix)?;
		Ok(PciBus {
			domain: self.bus.domain,
			bus,
		})
	}

	pub fn driver(&self) -> crate::AResult<Option<Driver>> {
		let link = self.device_file("driver");
		match fs::symlink_metadata(&link) {
			Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
			Err(e) => bail!("Couldn't locate driver for PCI device {}: {}", self, e),
			Ok(attr) => if !attr.file_type().is_symlink() {
				bail!("driver for PCI device {} not a symlink", self);
			},
		}
		let path = with_context!(("Couldn't follow driver symlink for PCI device {}", self),
			Ok(fs::canonicalize(link)?)
		)?;
		Ok(Some(Driver{path}))
	}
}

impl fmt::Display for PciEndpoint {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}:{}", self.bus, self.slot_function)
	}
}

impl str::FromStr for PciEndpoint {
	type Err = ::failure::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		// max len: 0000:00:00.0
		// short: 0:0.0

		ensure!(s.len() <= 12, "PCI endpoint too long: {:?}", s);

		let (domain, bus_s, devfun_s) = {
			let mut parts = s.split(':');
			let p1 = parts.next().ok_or_else(|| format_err!("Need at least one ':' in PCI endpoint: {:?}", s))?;
			let p2 = parts.next().ok_or_else(|| format_err!("Need at least one ':' in PCI endpoint: {:?}", s))?;
			match parts.next() {
				None => (0, p1, p2),
				Some(p3) => {
					ensure!(parts.next().is_none(), "At most two ':' in PCI endpoint: {:?}", s);

					let domain = with_context!(("invalid PCI domain: {}", p1),
						Ok(u16::from_str_radix(p1, 16)?)
					)?;

					(domain, p2, p3)
				}
			}
		};

		let bus = with_context!(("invalid PCI bus: {}", bus_s),
			Ok(u8::from_str_radix(bus_s, 16)?)
		)?;

		let slot_function = devfun_s.parse::<SlotFunction>()?;
		let bus = PciBus {
			domain,
			bus,
		};

		Ok(PciEndpoint {
			bus,
			slot_function,
		})
	}
}

#[derive(Debug)]
pub struct ScopedEnable {
	ep: Option<PciEndpoint>, // is none if already "closed" or was already enabled before
}

impl ScopedEnable {
	pub fn close(mut self) -> crate::AResult<()> {
		if let Some(ep) = self.ep.take() {
			ep.disable()?;
		}
		Ok(())
	}
}

impl Drop for ScopedEnable {
	fn drop(&mut self) {
		if let Some(ep) = self.ep.take() {
			if let Err(e) = ep.disable() {
				error!("PCI {}: Failed to disable temporarily enabled device: {}", ep, e);
			}
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct VendorId(pub u16);

impl fmt::Display for VendorId {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "0x{:04x}", self.0)
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DeviceID(pub u16);

impl fmt::Display for DeviceID {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "0x{:04x}", self.0)
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ClassCode(pub u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SubClassCode(pub u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ProgrammingInterface(pub u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Class {
	pub class_code: ClassCode,
	pub subclass_code: SubClassCode,
	pub programming_interface: ProgrammingInterface,
}

impl fmt::Display for Class {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"0x{:02x}{:02x}{:02x}",
			self.class_code.0,
			self.subclass_code.0,
			self.programming_interface.0,
		)
	}
}

#[cfg(test)]
mod test {
	use super::SlotFunction;

	fn check_dev_fun(dev: u8, fun: u8, repr: &str) {
		assert!(dev < 0x20);
		assert!(fun < 0x08);
		match repr.parse::<SlotFunction>() {
			Err(e) => panic!("{} failed to parse as SlotFunction: {}", repr, e),
			Ok(df) => assert_eq!(SlotFunction(dev << 3 | fun), df, "failed validing parsed {}", repr),
		}
	}

	fn check_dev_fun_canonical(dev: u8, fun: u8, repr: &str) {
		check_dev_fun(dev, fun, repr);
		assert_eq!(SlotFunction(dev << 3 | fun).to_string(), repr, "failed stringifying dev 0x{:02x} function {}", dev, fun);
	}

	fn check_invalid_dev_fun(repr: &str) {
		assert!(repr.parse::<SlotFunction>().is_err(), "{:?} must not be a valid DEV.FUN");
	}

	#[test]
	fn parse_dev_function() {
		check_dev_fun(0b0_0000, 0b000, "0.0");
		check_dev_fun_canonical(0b0_0000, 0b000, "00.0");
		check_dev_fun_canonical(0b0_0000, 0b001, "00.1");
		check_dev_fun_canonical(0b0_0000, 0b111, "00.7");
		check_dev_fun_canonical(0b0_0001, 0b000, "01.0");
		check_dev_fun_canonical(0b0_0001, 0b001, "01.1");
		check_dev_fun_canonical(0b0_0001, 0b111, "01.7");
		check_dev_fun_canonical(0b1_0000, 0b000, "10.0");
		check_dev_fun_canonical(0b1_0000, 0b111, "10.7");
		check_dev_fun_canonical(0b1_1111, 0b011, "1f.3");
		check_dev_fun_canonical(0b1_1111, 0b111, "1f.7");
		check_invalid_dev_fun("");
		check_invalid_dev_fun(".");
		check_invalid_dev_fun("0.");
		check_invalid_dev_fun("00.");
		check_invalid_dev_fun("000.");
		check_invalid_dev_fun(".0");
		check_invalid_dev_fun(".00");
		check_invalid_dev_fun(".000");
		check_invalid_dev_fun("0");
		check_invalid_dev_fun("00");
		check_invalid_dev_fun("000");
		check_invalid_dev_fun("0000");
	}
}
