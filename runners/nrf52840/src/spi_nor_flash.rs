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
		let mut buf: [u8; 256] = [0; 256];

		loop {
			buf.fill(0);
			buf[0] = 0x9f;

			let jedec_r = do_transfer(spi, &mut self.cs, &mut buf[0..12]);
			if jedec_r.is_ok() {
				if buf[1] != 0x00 && buf[1] != 0xff {
					rtt_target::rprintln!("ExtFlash JEDEC {:02x} {:02x} {:02x}", buf[1], buf[2], buf[3]);
					break;
				}
			} else {
				rtt_target::rprintln!("ExtFlash JEDEC Error");
				panic!();
			}
		}

		buf.fill(0);
		buf[0] = 0x5a;
		let sfdp_dump = do_transfer(spi, &mut self.cs, &mut buf);

		for i in 0..32 {
			rtt_target::rprintln!("SFDP HDR {:08x}", u32::from_le_bytes(buf[5+4*i..9+4*i].try_into().unwrap()));
		}
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


fn do_transfer<SPI, CS>(spi: &mut SPI, _cs: &mut CS, buf: &mut [u8]) -> Result<(), ()> where SPI: Transfer<u8>, CS: OutputPin {
	// cs.set_low().ok();

	let r = spi.transfer(buf).map(|_| ()).map_err(|_| ());

	// cs.set_high().ok();

	r
}
