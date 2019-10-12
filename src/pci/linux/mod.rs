use std::io;

mod config_space;
mod file;
mod mapped;
mod resource;

use self::mapped::Mapped;
use self::file::File;

use crate::pci::{
	PciEndpoint,
	PciConfigSpace,
	PciConfigSpaceReadOnly,
	PciResource,
	PciResourceReadOnly,
};

pub fn open_config_space_readonly(endpoint: PciEndpoint) -> io::Result<impl PciConfigSpaceReadOnly> {
	let path = format!("/sys/bus/pci/devices/{}/config", endpoint);
	file::inner_open(endpoint, path, false)
}

pub fn open_config_space_readwrite(endpoint: PciEndpoint) -> io::Result<impl PciConfigSpace> {
	let path = format!("/sys/bus/pci/devices/{}/config", endpoint);
	file::inner_open(endpoint, path, true)
}


pub fn open_resource_readonly(endpoint: PciEndpoint, resource: usize) -> io::Result<impl PciResourceReadOnly> {
	let path = format!("/sys/bus/pci/devices/{}/resource{}", endpoint, resource);
	mapped::inner_open(endpoint, path, false)
}

pub fn open_resource_readwrite(endpoint: PciEndpoint, resource: usize) -> io::Result<impl PciResource> {
	let path = format!("/sys/bus/pci/devices/{}/resource{}", endpoint, resource);
	mapped::inner_open(endpoint, path, true)
}
