use std::fs;
use std::io;

use super::PciEndpoint;

pub fn list_all_endpoints() -> io::Result<Vec<PciEndpoint>> {
	let mut list = Vec::new();
	for entry in fs::read_dir("/sys/bus/pci/devices")? {
		let entry = entry?;
		let fname = entry.file_name().into_string().map_err(|e| {
			io::Error::new(io::ErrorKind::Other, format!("Invalid (Non-UTF8) PCI device name {:?}", e))
		})?;
		let ep = fname.parse::<PciEndpoint>().map_err(|e| {
			io::Error::new(io::ErrorKind::Other, format!("Invalid PCI device name: {}", e))
		})?;
		list.push(ep);
	}

	Ok(list)
}
