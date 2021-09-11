use crate::spi_nor_flash::SpiNorFlash;
use embedded_hal::blocking::spi::Transfer;
use nrf52840_hal::{
	gpio::{Output, Pin, PushPull},
	prelude::OutputPin
};

#[cfg(feature = "board-proto1")]
pub const FLASH_SIZE: usize = 0x20_0000;	/* GD25Q16C, 16 Mbit == 2 MB */
#[cfg(feature = "board-nrfdk")]
pub const FLASH_SIZE: usize = 0x80_0000;	/* MX25R6435F, 64 Mbit == 8 MB */

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
