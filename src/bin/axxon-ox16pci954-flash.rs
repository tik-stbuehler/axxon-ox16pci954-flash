#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

extern crate axxon_ox16pci954_flash;
use axxon_ox16pci954_flash::*;

use std::process::exit;

fn main_app() -> AResult<()> {
	let matches = clap_app!(@app (app_from_crate!())
		(global_setting: clap::AppSettings::VersionlessSubcommands)
		(@arg flash: --flash "Flash devices (if not using target images already)")
	).get_matches();
	let flash_devices = matches.is_present("flash");
	let mut need_flashing = false;

	let mut ox16pci954_busses = std::collections::HashSet::new();
	// list of endpoints (function 1) that should be checked because function 0 was in use
	let mut ox16pci954_check_f1 = std::collections::HashSet::new();
	let mut all = pci::list_all_endpoints()?;
	all.sort();
	for ep in all {
		if axxon::is_pex8112_bridge(ep)? {
			let _se = ep.scoped_enable()?;
			let s = pci::open_config_space_readwrite(ep)?;
			let mut flash = match axxon::open_flash(s) {
				Err(e) => {
					error!("PCI {}: probably not an AXXON device: {:?}", ep, e);
					continue;
				},
				Ok(f) => f,
			};

			let bridge_image = match axxon::extract_image(&mut flash) {
				Err(e) => {
					error!("PCI {}: failed to read image: {:?}", ep, e);
					break;
				}
				Ok(i) => i,
			};

			if bridge_image != &axxon::IMAGE[..] {
				info!("PCI {}: Axxon PCI bridge image not up to date", ep);
				if flash_devices {
					if let Err(e) = axxon::write_image(&mut flash, &axxon::IMAGE) {
						error!("PCI {}: Failed to flash Axxon PCI bridge image: {}", ep, e);
						bail!("Failed to flash");
					}
				} else {
					need_flashing = true;
				}
			} else {
				info!("PCI {}: Axxon PCI bridge image up to date", ep);
			}

			let bus = ep.secondary_bus()?;
			if bus < ep.bus {
				error!("PCI {}: Bridge has a secondary bus ({}) with an id less than its own, won't find OX16PCI954 devices", ep, bus);
			}
			ox16pci954_busses.insert(bus);
		} else if ox16_pci954::is_ox16_pci954(ep)? {
			let _se = ep.scoped_enable()?;
			let is_axxon_card = ox16pci954_busses.contains(&ep.bus);
			if !is_axxon_card {
				warn!("PCI {}: Found OX16PCI954 device, but not behind an Axxon PCIe-to-PCI bridge", ep);
			} else {
				info!("PCI {}: Found OX16PCI954 device on Axxon card", ep);
			}

			if let Some(driver) = ep.driver()? {
				if !is_axxon_card {
					warn!("PCI {}: Not checking flash, as OX16PCI954 is in use by driver {} (and this is not an Axxon card)", ep, driver);
					continue;
				} else if ep.slot_function.function() == 0 {
					info!("PCI {}: Not checking flash on function 0 as it is in use by driver {} (function 1 will be using the same flash though)", ep, driver);
					let mut ep_f1 = ep;
					ep_f1.slot_function.0 += 1;
					ox16pci954_check_f1.insert(ep_f1);
					continue;
				} else {
					// there shouldn't be any driver on function 1, as UARTs are only on function 0, and function 1 should be disabled on Axxon cards
					warn!("PCI {}: In use by driver {}, but shouldn't: the device function isn't wired. Unbinding driver.", ep, driver);
					driver.unbind(ep)?;
				}
			}
			ox16pci954_check_f1.remove(&ep);

			let mut ee = ox16_pci954::open_eeprom(ep)?;
			let image = ox16_pci954::read_flash_program(&mut ee)?;
			if image != &ox16_pci954::IMAGE[..] {
				info!("PCI {}: OX16PCI954 image not up to date", ep);
				if flash_devices {
					if let Err(e) = ox16_pci954::flash_program(&mut ee, &ox16_pci954::IMAGE) {
						error!("PCI {}: Failed to flash OX16PCI954 image: {}", ep, e);
						bail!("Failed to flash");
					}
				} else {
					need_flashing = true;
				}
			} else {
				info!("PCI {}: OX16PCI954 image up to date", ep);
			}
		}
	}

	for ep in ox16pci954_check_f1 {
		error!("PCI {}: wasn't checked, but we skipped function 0 because a driver was loaded", ep);
	}

	if need_flashing {
		info!("One or multiple devices are not using the target images");
		exit(11);
	}

	Ok(())
}

fn main() {
	env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

	if let Err(e) = main_app() {
		error!("Error: {}", e);
		// eprintln!("Backtrace: {:?}", e.backtrace());
		exit(1);
	}
}
