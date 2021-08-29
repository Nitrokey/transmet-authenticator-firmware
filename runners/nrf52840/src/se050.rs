use nrf52840_hal::{
	gpio::{Pin, Output, PushPull},
	prelude::OutputPin,
	twim::Twim,
};
use rtt_target;

const I2CS_SE050_ADDRESS: u8 = 0x48;

pub struct Se050<T> {
	twi: Twim<T>,
	power_pin: Pin<Output<PushPull>>,
}

impl<T> Se050<T> where T: nrf52840_hal::twim::Instance {

	pub fn new(twi: Twim<T>, pwr_pin: Pin<Output<PushPull>>) -> Se050<T> {
		Se050 { twi: twi, power_pin: pwr_pin }
	}

	pub fn enable(&mut self) {
		// let mut txbuf: [u8; _] = [0x80, 0x04, 0x00, 0x20, 
		let mut txbuf: [u8; 0x16] = [0x00, 0xa4, 0x04, 0x00, 0x10,
				0xA0, 0x00, 0x00, 0x03, 0x96, 0x54, 0x53, 0x00,
				0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00,
				0x00];
		let mut rxbuf: [u8; 9] = [0u8; 9];
		self.power_pin.set_high().ok();
		self.twi.write_then_read(I2CS_SE050_ADDRESS, &txbuf, &mut rxbuf).ok();
		rtt_target::rprintln!("SE050 R-APDU: {:x}{:x}", rxbuf[7], rxbuf[8]);
	}
}
