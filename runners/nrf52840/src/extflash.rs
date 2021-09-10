use crate::spi_nor_flash::SpiNorFlash;
use embedded_hal::blocking::spi::Transfer;
use nrf52840_hal::{
	gpio::{Output, Pin, PushPull},
	prelude::OutputPin
};

pub const FLASH_SIZE: usize = 0x2_0000;

pub struct ExtFlashStorage<SPI> where SPI: Transfer<u8> {
	// extflash: spi_memory::series25::Flash<SPI, Pin<Output<PushPull>>>,
	extflash: SpiNorFlash<SPI, Pin<Output<PushPull>>>,
	power_pin: Option<Pin<Output<PushPull>>>,
}

impl<SPI> littlefs2::driver::Storage for ExtFlashStorage<SPI> where SPI: Transfer<u8> {

	const BLOCK_SIZE: usize = 4096;
	const READ_SIZE: usize = 4;
	const WRITE_SIZE: usize = 4;
	const BLOCK_COUNT: usize = FLASH_SIZE / Self::BLOCK_SIZE;
	type CACHE_SIZE = generic_array::typenum::U256;
	type LOOKAHEADWORDS_SIZE = generic_array::typenum::U1;

	// the ReadNorFlash trait exposes a try_read() which (stupidly) expects a mutable self
	// can't get those two to align - so clone the function and drop the mut there
	fn read(&self, off: usize, buf: &mut [u8]) -> Result<usize, littlefs2::io::Error> {
		// rtt_target::rprintln!("F RD {:x} {:x}", off, buf.len());
		Err(littlefs2::io::Error::Unknown(0x6565_6565))
	}

	fn write(&mut self, off: usize, buf: &[u8]) -> Result<usize, littlefs2::io::Error> {
		// rtt_target::rprintln!("F WR {:x} {:x}", off, buf.len());
		Err(littlefs2::io::Error::Unknown(0x6565_6565))
	}

	fn erase(&mut self, off: usize, len: usize) -> Result<usize, littlefs2::io::Error> {
		// rtt_target::rprintln!("F ER {:x} {:x}", off, len);
		Err(littlefs2::io::Error::Unknown(0x6565_6565))
	}
}

/*
impl<SPI> ExtFlashStorage<SPI> where SPI: Transfer<u8> {

	pub fn new(spim: SPI, cs: Pin<Output<PushPull>>, mut power_pin: Pin<Output<PushPull>>) -> Option<Self> {
		power_pin.set_high().ok();

		let extflash_r = spi_memory::series25::Flash::init(spim, cs);

		if let Ok(mut extflash) = extflash_r {
			Some(Self { extflash, power_pin })
		} else {
			None
		}
	}

	pub fn debug_ident(&mut self) {
		let status_r = self.extflash.read_status();
		let jedec_r = self.extflash.read_jedec_id();
		match status_r {
		Err(spi_memory::Error::Spi(_)) => { rtt_target::rprintln!("Status: SPI Error"); }
		Err(spi_memory::Error::Gpio(_)) => { rtt_target::rprintln!("Status: GPIO Error"); }
		Err(_) => { rtt_target::rprintln!("Status: Other Error"); }
		Ok(status) => { rtt_target::rprintln!("Status {:?}", status); }
		}
		match jedec_r {
		Err(spi_memory::Error::Spi(_)) => { rtt_target::rprintln!("JEDEC: SPI Error"); }
		Err(spi_memory::Error::Gpio(_)) => { rtt_target::rprintln!("JEDEC: GPIO Error"); }
		Err(_) => { rtt_target::rprintln!("JEDEC: Other Error"); }
		Ok(jedec_id) => { rtt_target::rprintln!("JEDEC {:?}", jedec_id); }
		}
	}
}
*/

impl<SPI> ExtFlashStorage<SPI> where SPI: Transfer<u8> {

	pub fn new(spim: &mut SPI, cs: Pin<Output<PushPull>>, power_pin: Option<Pin<Output<PushPull>>>) -> Self {
		let extflash = SpiNorFlash::new(spim, cs);
		Self { extflash, power_pin }
	}

	pub fn init(&mut self, spim: &mut SPI) {
		self.power_up();
		self.extflash.init(spim);
	}

	fn power_up(&mut self) {
		if let Some(mut pwr_pin) = self.power_pin.as_mut() {
			pwr_pin.set_high().ok();
		}
	}

}
