#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

extern crate axxon_ox16pci954_flash;
use axxon_ox16pci954_flash::*;

use std::io::{
	self,
	Write,
};
use std::process::exit;

use axxon_ox16pci954_flash::pci::PciResourceReadOnly;

fn get_param<T>(matches: &clap::ArgMatches, name: &str) -> AResult<T>
where
	T: std::str::FromStr,
	failure::Error: From<<T as std::str::FromStr>::Err>,
{
	let param = match matches.value_of(name) {
		Some(p) => p,
		None => bail!("missing parameter {}", name),
	};
	param.parse::<T>().map_err(|e| {
		let e = failure::Error::from(e);
		let msg = format!("invalid paramater {}: {}", name, e);
		e.context(msg).into()
	})
}

fn with_resources_dev<F, R>(ep: pci::PciEndpoint, allow_unbind: bool, f: F) -> AResult<Option<R>>
where
	F: FnOnce() -> AResult<R>,
{
	let enabled = ep.is_enabled()?;

	if !enabled {
		ep.enable()?;
	}

	let restore_driver = match ep.driver()? {
		None => None,
		Some(driver) => {
			if !allow_unbind {
				eprintln!("can't use device while bound to driver {:?}", driver);
				return Ok(None);
			}

			driver.unbind(ep)?;

			Some(driver)
		}
	};

	let res = f();

	if let Some(driver) = restore_driver {
		if let Err(e) = driver.bind(ep) {
			eprintln!("Failed rebinding device {} to driver {}: {}", ep, driver, e);
			// TODO: set non-zero exit code?
		}
	}

	if !enabled {
		if let Err(e) = ep.disable() {
			eprintln!("Failed to disabled device {}: {}", ep, e);
			// TODO: set non-zero exit code?
		}
	}

	res.map(Some)
}

fn dump_resource(sub_m: &clap::ArgMatches) -> AResult<()> {
	let ep: pci::PciEndpoint = get_param(sub_m, "DEVICE")?;
	let resource: usize = get_param(sub_m, "RESOURCE")?;
	let allow_unbind = sub_m.is_present("unbind");

	if with_resources_dev(ep, allow_unbind, || {
		let res = pci::open_resource_readonly(ep, resource)?;

		io::stdout().write(&res.read_into_vec())?;

		Ok(())
	})?.is_none() {
		exit(1);
	}

	Ok(())
}

fn list_all() -> AResult<()> {
	let mut all = pci::list_all_endpoints()?;
	all.sort();
	for ep in all {
		println!("{}", ep);
	}

	Ok(())
}

fn list_ox16_pci954() -> AResult<()> {
	let mut all = pci::list_all_endpoints()?;
	all.sort();
	for ep in all {
		if !ox16_pci954::is_ox16_pci954(ep)? {
			continue;
		}

		println!("{}", ep);
	}

	Ok(())
}

fn info(sub_m: &clap::ArgMatches) -> AResult<()> {
	let ep: pci::PciEndpoint = get_param(sub_m, "DEVICE")?;
	let allow_unbind = sub_m.is_present("unbind");

	if !ox16_pci954::is_ox16_pci954(ep)? {
		eprintln!("Device {} is not an OX16PCI954 PCI device", ep);
		exit(1);
	}

	if with_resources_dev(ep, allow_unbind, || {
		println!("{:?}", ox16_pci954::decode_resource3(ep)?);

		Ok(())
	})?.is_none() {
		exit(1);
	}

	Ok(())
}

fn dump_eeprom(sub_m: &clap::ArgMatches) -> AResult<()> {
	let ep: pci::PciEndpoint = get_param(sub_m, "DEVICE")?;
	let allow_unbind = sub_m.is_present("unbind");

	if !ox16_pci954::is_ox16_pci954(ep)? {
		eprintln!("Device {} is not an OX16PCI954 PCI device", ep);
		exit(1);
	}

	if with_resources_dev(ep, allow_unbind, || {
		use axxon_ox16pci954_flash::serial::HardwareOperations;

		let mut ee = ox16_pci954::open_eeprom(ep)?;
		for (address, word) in ee.read_all()?.enumerate() {
			println!("@{:02x}: {:04x}", address, word);
		}

		Ok(())
	})?.is_none() {
		exit(1);
	}

	Ok(())
}

fn axxon_verify_eeprom(sub_m: &clap::ArgMatches) -> AResult<()> {
	let ep: pci::PciEndpoint = get_param(sub_m, "DEVICE")?;

	if ep.vendor()?.0 != 0x10B5 || ep.device()?.0 != 0x8112 {
		eprintln!("Device {} is not an Axxon PCI device", ep);
		exit(1);
	}

	let image = with_configspace_dev(ep, || {
		let s = pci::open_config_space_readwrite(ep)?;
		let mut flash = axxon::open_flash(s)?;

		axxon::extract_image(&mut flash)
	})?;

	if &image[..] == &axxon::IMAGE[..] {
		println!("Image verified successfully");
	} else {
		eprintln!("Unexpected flash image data:");
		for i in 0..image.len() {
			if 0 == i % 16 {
				eprint!("{:08x} ", i);
			} else if 0 == i % 8 {
				eprint!(" ");
			}
			eprint!(" {:02x}", image[i]);
			if 15 == i % 16 {
				eprintln!("");
			}
		}
		if 0 != image.len() % 16 {
			eprintln!("");
		}
		eprintln!("{:08x}", image.len());
	}

	Ok(())
}

fn axxon_dump_eeprom(sub_m: &clap::ArgMatches) -> AResult<()> {
	let ep: pci::PciEndpoint = get_param(sub_m, "DEVICE")?;

	if ep.vendor()?.0 != 0x10B5 || ep.device()?.0 != 0x8112 {
		eprintln!("Device {} is not an Axxon PCI device", ep);
		exit(1);
	}

	with_configspace_dev(ep, || {
		let s = pci::open_config_space_readwrite(ep)?;
		let mut flash = axxon::open_flash_recovery(s)?;

		let image = axxon::extract_image(&mut flash)?;
		io::stdout().write(&image)?;

		Ok(())
	})?;

	Ok(())
}

fn main_app() -> AResult<()> {
	let matches = clap_app!(@app (app_from_crate!())
		(@setting SubcommandRequiredElseHelp)
		(global_setting: clap::AppSettings::VersionlessSubcommands)
		(@subcommand list =>
			(about: "list OX16PCI954 PCI devices")
		)
		(@subcommand info =>
			(about: "show info for OX16PCI954 PCI device")
			(@arg unbind: -u --unbind "temporarily unbind driver if present")
			(@arg DEVICE: +required "PCI device to use ([bus:]slot:dev.fun)")
		)
		(@subcommand dump_eeprom =>
			(about: "dump EEPROM for OX16PCI954 PCI device")
			(@arg unbind: -u --unbind "temporarily unbind driver if present")
			(@arg DEVICE: +required "PCI device to use ([bus:]slot:dev.fun)")
		)
		(@subcommand list_all =>
			(about: "list all PCI devices")
		)
		(@subcommand dump_resource =>
			(about: "dumps PCI resource region")
			(@arg unbind: -u --unbind "temporarily unbind driver if present")
			(@arg DEVICE: +required "PCI device to use ([bus:]slot:dev.fun)")
			(@arg RESOURCE: +required "Resource number to dump")
		)
		(@subcommand axxon =>
			(about: "Axxon PCI device commands")
			(@setting SubcommandRequiredElseHelp)
			(@subcommand verify =>
				(about: "verify flash image")
				(@arg DEVICE: +required "PCI device to use ([bus:]slot:dev.fun)")
			)
			(@subcommand dump_eeprom =>
				(about: "dump EEPROM for AXXON PCI device as binary to stdout")
				(@arg DEVICE: +required "PCI device to use ([bus:]slot:dev.fun)")
			)
		)
	).get_matches();

	match matches.subcommand() {
		("list", _) => {
			list_ox16_pci954()
		}
		("info", Some(sub_m)) => {
			info(sub_m)
		}
		("dump_eeprom", Some(sub_m)) => {
			dump_eeprom(sub_m)
		}
		("list_all", _) => {
			list_all()
		}
		("dump_resource", Some(sub_m)) => {
			dump_resource(sub_m)
		},
		("axxon", Some(sub_m)) => match sub_m.subcommand() {
			("verify", Some(sub_sub_m)) => {
				axxon_verify_eeprom(sub_sub_m)
			},
			("dump_eeprom", Some(sub_m)) => {
				axxon_dump_eeprom(sub_m)
			}
			("", _) => bail!("no subcommand"),
			(cmd, _) => bail!("not implemented subcommand for 'axxon' {:?}", cmd),
		},
		("", _) => bail!("no subcommand"),
		(cmd, _) => bail!("not implemented subcommand {:?}", cmd),
	}
}

fn main() {
	env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

	if let Err(e) = main_app() {
		error!("Error: {}", e);
		// eprintln!("Backtrace: {:?}", e.backtrace());
		exit(1);
	}
}
