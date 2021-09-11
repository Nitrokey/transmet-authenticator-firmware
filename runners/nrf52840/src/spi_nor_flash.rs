use core::convert::TryInto;
use embedded_hal::{
	digital::v2::OutputPin,
	blocking::spi::Transfer
};

pub struct SpiNorFlash<SPI, CS> where SPI: Transfer<u8>, CS: OutputPin {
	cs: CS,
	_spi: core::marker::PhantomData<SPI>
}

impl<SPI, CS> SpiNorFlash<SPI, CS> where SPI: Transfer<u8>, CS: OutputPin {

	pub fn new(spi: &mut SPI, cs: CS) -> Self {
		Self { cs, _spi: core::marker::PhantomData { } }
	}

	pub fn init(&mut self, spi: &mut SPI) {
		let mut buf: [u8; 384] = [0; 384];

		loop {
			buf.fill(0);
			buf[0] = 0x9f;

			let jedec_r = do_transfer(spi, &mut self.cs, &mut buf[0..12]);
			if jedec_r {
				if buf[1] != 0x00 && buf[1] != 0xff {
					rtt_target::rprintln!("ExtFlash JEDEC {:02x} {:02x} {:02x}", buf[1], buf[2], buf[3]);
					break;
				}
			} else {
				rtt_target::rprintln!("ExtFlash JEDEC Error");
				panic!();
			}
		}
/*
		buf.fill(0);
		buf[0] = 0x5a;
		let sfdp_dump = do_transfer(spi, &mut self.cs, &mut buf);

		for i in 0..(384-5)/8 {
			rtt_target::rprintln!("SFDP HDR[{:03x}] {:08x} {:08x}", i<<3,
				u32::from_le_bytes(buf[5+8*i..9+8*i].try_into().unwrap()),
				u32::from_le_bytes(buf[9+8*i..13+8*i].try_into().unwrap()));
		}
*/
		buf.fill(0);
		buf[0] = 0x03;
		let read_data = do_transfer(spi, &mut self.cs, &mut buf);
		rtt_target::rprintln!("Data: {:x} {:x} {:x}...", buf[0], buf[19], buf[83]);
	}

	fn read_sfdp(&mut self, spi: &mut SPI, index: u32) -> (u32, u32) {
		let mut buf: [u8; 17] = [0u8; 17];
		buf[0] = 0x5a;

		let byteindex = index << 3;
		buf[1] = (byteindex >> 16) as u8;
		buf[2] = (byteindex >>  8) as u8;
		buf[3] =  byteindex        as u8;

		let sfdp_r = do_transfer(spi, &mut self.cs, &mut buf);

		( u32::from_le_bytes(buf[5..9].try_into().unwrap()),
		  u32::from_le_bytes(buf[9..13].try_into().unwrap()) )
	}
}


fn do_transfer<SPI, CS>(spi: &mut SPI, _cs: &mut CS, buf: &mut [u8]) -> bool where SPI: Transfer<u8>, CS: OutputPin {
	// cs.set_low().ok();
	let r = spi.transfer(buf);
	// cs.set_high().ok();
	r.is_ok()
}
