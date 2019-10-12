use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{
	Path,
	PathBuf,
};

use super::PciEndpoint;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Driver {
	pub(super) path: PathBuf,
}

impl fmt::Display for Driver {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self.path.file_name().unwrap())
	}
}

impl Driver {
	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn bind(&self, ep: PciEndpoint) -> crate::AResult<()> {
		// need to write in one syscall for unbind/bind
		let ep_str = ep.to_string();

		with_context!(("bind {} to driver {}", ep_str, self), {
			fs::OpenOptions::new().write(true).open(self.path.join("bind"))?.write_all(ep_str.as_bytes())?;

			Ok(())
		})
	}

	pub fn unbind(&self, ep: PciEndpoint) -> crate::AResult<()> {
		// need to write in one syscall for unbind/bind
		let ep_str = ep.to_string();

		with_context!(("unbind {} from driver {}", ep_str, self), {
			fs::OpenOptions::new().write(true).open(self.path.join("unbind"))?.write_all(ep_str.as_bytes())?;

			Ok(())
		})
	}
}
