use nrf52840_hal::{
	gpio::{Input, Output, Pin, PullDown, PushPull},
	prelude::{OutputPin},
	uarte::Uarte,
};
use rtt_target;

const FPR_MAGIC: u16 = 0xef01;
const FPR_ADDRESS: u32 = 0xffff_ffff;

pub enum FPRError {
	InitFailed,
	ReadError,
	WriteError,
	BufferOverrun,
	HeaderError,
	ChecksumError,
	UnknownError
}

pub struct FingerprintReader<T> {
	uart: Uarte<T>,
	power_pin: Pin<Output<PushPull>>,
	detect_pin: Pin<Input<PullDown>>
}

impl<T> FingerprintReader<T> where T: nrf52840_hal::uarte::Instance {

	pub fn new(uart: Uarte<T>, pwr_pin: Pin<Output<PushPull>>, det_pin: Pin<Input<PullDown>>) -> Self {
		Self { uart, power_pin: pwr_pin, detect_pin: det_pin }
	}

	pub fn power_up(&mut self) -> Result<(), FPRError> {
		self.power_pin.set_low().ok();
		let mut ready: [u8; 1] = [0];

		rtt_target::rprintln!("FPR: on, awaiting ready");
		self.uart.read(&mut ready).ok();
		if ready[0] != 0x55 {
			return Err(FPRError::InitFailed);
		}

		rtt_target::rprintln!("FPR: setting PLC");
		let ssp_plc: [u8; 3] = [0x0e, 6, 3];		/* Packet Length Coeff. -> 3 (Data Packets 256B each) */
		self.command(&ssp_plc, &mut ready)?;
		rtt_target::rprintln!("FPR: PLC response {:02x}", ready[0]);

		Ok(())
	}

	pub fn power_down(&mut self) -> Result<(), FPRError> {
		rtt_target::rprintln!("FPR: off");
		self.power_pin.set_high().ok();
		Ok(())
	}

	pub fn check_detect(&self, latches: &[u32]) -> bool {
		crate::types::is_pin_latched(&self.detect_pin, latches)
	}

	fn command(&mut self, cmd: &[u8], resp: &mut [u8]) -> Result<(), FPRError> {
		let mut cmdbuf: [u8; 64] = [0; 64];
		let mut rsphdr: [u8; 9] = [0; 9];
		let mut rspbuf: [u8; 64] = [0; 64];

		let clen = cmd.len() + 2;
		if 9+clen > 64 {
			return Err(FPRError::BufferOverrun);
		}

		cmdbuf[0] = (FPR_MAGIC >> 8) as u8;
		cmdbuf[1] = (FPR_MAGIC >> 0) as u8;
		cmdbuf[2] = (FPR_ADDRESS >> 24) as u8;
		cmdbuf[3] = (FPR_ADDRESS >> 16) as u8;
		cmdbuf[4] = (FPR_ADDRESS >>  8) as u8;
		cmdbuf[5] = (FPR_ADDRESS >>  0) as u8;
		cmdbuf[6] = 0x01;		/* COMMAND */
		cmdbuf[7] = (clen >> 8) as u8;
		cmdbuf[8] = (clen >> 0) as u8;
		for i in 0..(clen-2) {
			cmdbuf[9+i] = cmd[i];
		}
		let chk: u16 = sum_up(&cmdbuf[6..9+clen-2], 0);
		cmdbuf[9+clen-2] = (chk >> 8) as u8;
		cmdbuf[9+clen-1] = (chk >> 0) as u8;

		self.uart.write(&cmdbuf[0..(9+clen)]).map_err(|_| FPRError::WriteError)?;
		self.uart.read(&mut rsphdr).map_err(|_| FPRError::ReadError)?;

		for i in 0..6 {
			if rsphdr[i] != cmdbuf[i] {
				return Err(FPRError::HeaderError);
			}
		}

		// TODO: check for packet types (continuation / final data packets)

		rtt_target::rprintln!("_fpr rsp {:02x} {:02x}{:02x}", rsphdr[6], rsphdr[7], rsphdr[8]);
		let rsplen: usize = (((rsphdr[7] as u16) << 8) | (rsphdr[8] as u16)) as usize;
		if rsplen > 64 {
			return Err(FPRError::BufferOverrun);
		}
		self.uart.read(&mut rspbuf[0..rsplen]).map_err(|_| FPRError::ReadError)?;

		let rchk = sum_up(&rspbuf[0..rsplen-2], sum_up(&rsphdr[6..9], 0));
		if (rspbuf[rsplen-2] != ((rchk >> 8) as u8)) || (rspbuf[rsplen-1] != (rchk as u8)) {
			return Err(FPRError::ChecksumError);
		}

		for i in 0..core::cmp::min(resp.len(),rsplen-2) {
			resp[i] = rspbuf[i];
		}

		Ok(())
	}
}

fn sum_up(b: &[u8], iv: u16) -> u16 {
	let mut chksum: u16 = iv;

	for i in 0..b.len() {
		chksum += b[i] as u16;
	}

	chksum
}
