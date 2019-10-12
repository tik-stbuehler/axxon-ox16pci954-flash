mod config_space;
mod driver;
mod endpoint;
mod list;
mod linux;
mod resource;

pub use self::config_space::{
	PciConfigSpace,
	PciConfigSpaceReadOnly,
};

pub use self::driver::{
	Driver,
};

pub use self::endpoint::{
	SlotFunction,
	PciEndpoint,
};

pub use self::list::{
	list_all_endpoints,
};

pub use self::resource::{
	PciResource,
	PciResourceReadOnly,
};

// OS-specific. for now linux only.
pub use self::linux::{
	open_config_space_readonly,
	open_config_space_readwrite,
	open_resource_readonly,
	open_resource_readwrite,
};
