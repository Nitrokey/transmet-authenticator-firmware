use nrf52840_hal::{
	gpio::{Pin, Output, PushPull},
	prelude::{_embedded_hal_blocking_delay_DelayMs, OutputPin},
	twim::Twim,
};
use rtt_target;
use asm_delay::bitrate::*;

const I2CS_SE050_ADDRESS: u8 = 0x48;

pub struct Se050<T> {
	twi: Twim<T>,
	power_pin: Pin<Output<PushPull>>,
}

#[allow(dead_code,non_camel_case_types)]
pub enum T1_S_CODES {
	RESYNC		= 0b00000,
	IFS		= 0b00001,
	ABORT		= 0b00010,
	WTX		= 0b00011,
	END_APDU_SESSION= 0b00101,
	CHIP_RESET	= 0b00110,
	GET_ATR		= 0b00111,
	IF_SOFT_RESET	= 0b01111,
}

// T=1: NAD PCB LEN INF(*LEN) CRC16
// NAD: HD->SE 0x5a
// NAD: SE->HD 0xa5
// PCB-I: 0b0nm00000
// PCB-R: 0b100n00ee
// PCB-S: 0b11sssssq

// CRC: poly 1021, init direct FFFF, final xor FFFF, rev. input, rev. result
// (CRC16_X_25)

// error response: a5 82 00 da 4f
// correct INTF RESET REQ: 5a cf 00 37 7f
// SE050 INTF RESET RESPONSE: (ATF wrapped in T=1 packet)
// 	a5 ef 23
//		00
//		a0 00 00 03 96				(Application Provider: NXP)
//		04 03 e8 00 fe				(DL: BWT = 1000, IFSC = 254)
//		02					(DL Type: I2C)
//		0b 03 e8 08 01 00 00 00 00 64 00 00	(Phys. L.: Max.Clock = 1000, Conf = RFU3, MPOT = 1, RFU = {0,0,0}, SEGT = 64us, WUT = 0us)
//		0a 4a 43 4f 50 34 20 41 54 50 4f	(Hist.: "JCOP4 ATPO")
//	87 77

impl<T> Se050<T> where T: nrf52840_hal::twim::Instance {


	pub fn new(twi: Twim<T>, pwr_pin: Pin<Output<PushPull>>) -> Se050<T> {
		Se050 { twi: twi, power_pin: pwr_pin }
	}

	pub fn enable(&mut self) {
		let mut delay_provider = asm_delay::AsmDelay::new(64_u32.mhz());

		let mut txbuf: [u8; 5] = [0x5a, 0b11000000 | (0 << 5) | (T1_S_CODES::IF_SOFT_RESET as u8), 0x00, 0x37, 0x7f];
		let mut rxbuf: [u8; 48] = [0u8; 48];
		self.power_pin.set_high().ok();
		delay_provider.delay_ms(1u32);
		self.twi.write(I2CS_SE050_ADDRESS, &txbuf).ok();
		delay_provider.delay_ms(1u32);
		self.twi.read(I2CS_SE050_ADDRESS, &mut rxbuf[0..3]).ok();
		rtt_target::rprintln!("SE050 R-APDU: {:x} {:x} {:x}", rxbuf[0], rxbuf[1], rxbuf[2]);
		if rxbuf[0] == 0xa5 {
			let rlen: usize = (rxbuf[2] + 2) as usize;
			self.twi.read(I2CS_SE050_ADDRESS, &mut rxbuf[0..rlen]).ok();
		}
	}
}
