use std::thread;
use std::time::{
	Duration,
	Instant,
};

const CLOCK_EDGE: Duration = Duration::from_nanos(250);
// const CLOCK_FULL: Duration = Duration::from_nanos(500);

pub fn reliable_sleep(mut duration: Duration) {
	loop {
		let now = Instant::now();
		thread::sleep(duration);
		let elapsed = now.elapsed();
		if elapsed >= duration {
			return;
		}
		duration -= elapsed;
	}
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OutPins {
	pub chip_select: bool,
	pub clock: bool,
	pub data: bool,
}

pub trait Hardware {
	fn set_pins(&mut self, pins: OutPins);
	fn read_pin(&mut self) -> bool;

	// delay for (at least) one clock edge
	fn delay(&mut self) {
		reliable_sleep(CLOCK_EDGE);
	}
}
