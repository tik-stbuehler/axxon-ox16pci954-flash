#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

macro_rules! with_context {
	(( $fmt:tt $($t:tt)* ), $e:expr) => {{
		use failure::Error;

		match (|| { $e })() {
			Ok(v) => Ok(v),
			Err(e) => {
				let e: Error = e;
				let msg = format!(concat!($fmt, ": {}") $($t)*, e);
				Err(Error::from(e.context(msg)))
			}
		}
	}};

	($msg:expr, $e:expr) => {
		with_context!(("{}", $msg), $e)
	};
}

pub type AResult<T> = Result<T, failure::Error>;

pub mod axxon;
pub mod serial;
pub mod ox16_pci954;
pub mod pci;

pub fn with_configspace_dev<F, R>(ep: pci::PciEndpoint, f: F) -> AResult<R>
where
	F: FnOnce() -> AResult<R>,
{
	let _se = ep.scoped_enable()?;
	f()
}
