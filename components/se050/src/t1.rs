use crate::types::*;
use core::convert::{Into, TryInto};

pub trait T1Proto {
	fn send_apdu(&mut self, apdu: &Apdu) -> Result<(), T1Error>;
	fn receive_apdu<'a>(&mut self, buf: &'a mut [u8], apdu: &'a mut Apdu) -> Result<(), T1Error>;
	fn interface_soft_reset(&mut self) -> Result<AnswerToReset, T1Error>;
}

pub struct T1overI2C<TWI> where TWI: embedded_hal::blocking::i2c::Read + embedded_hal::blocking::i2c::Write {
	twi: TWI,
	se_address: u16,
	nad_hd2se: u8,
	nad_se2hd: u8,
	iseq_snd: u8,
	iseq_rcv: u8,
}

impl<TWI> T1overI2C<TWI> where TWI: embedded_hal::blocking::i2c::Read + embedded_hal::blocking::i2c::Write {
	pub fn new(twi: TWI, address: u16, nad: u8) -> Self {
		let nad_r: u8 = ((nad & 0xf0) >> 4) | ((nad & 0x0f) << 4);
		T1overI2C { twi, se_address: address, nad_hd2se: nad, nad_se2hd: nad_r, iseq_snd: 0, iseq_rcv: 0 }
	}

	fn send_s(&mut self, code: T1SCode, data: &[u8]) -> Result<(), T1Error> {
		let mut buf: [u8; 260] = [0u8; 260];

		buf[0] = self.nad_hd2se;
		buf[1] = T1_S_REQUEST_CODE | <T1SCode as Into<u8>>::into(code);
		buf[2] = data.len() as u8;
		for i in 0..data.len() {
			buf[3+i] = data[i];
		}
		let crc: u16 = crc16_ccitt_oneshot(&buf[0..3+data.len()]);
		set_u16_le(&mut buf[3+data.len()..3+data.len()+2], crc);

		self.twi.write(self.se_address as u8, &buf[0..3+data.len()+2]).map_err(|_| T1Error::TransmitError)
	}

	fn receive_s(&mut self, code: T1SCode, data: &mut [u8]) -> Result<(), T1Error> {
		self.twi.read(self.se_address as u8, &mut data[0..3]).map_err(|_| T1Error::ReceiveError)?;
		if data[0] != self.nad_se2hd {
			return Err(T1Error::ProtocolError);
		}
		if data[1] != T1_S_RESPONSE_CODE | <T1SCode as Into<u8>>::into(code) {
			if (data[1] & T1_R_CODE_MASK) == T1_R_CODE {
				return Err(T1Error::RCodeReceived(data[1]));
			}
			return Err(T1Error::ProtocolError);
		}
		let dlen: usize = data[2] as usize;
		let mut crc: u16 = crc16_ccitt_init();
		crc = crc16_ccitt_update(crc, &data[0..3]);

		if dlen+2 > data.len() {
			return Err(T1Error::BufferOverrunError(dlen));
		}

		self.twi.read(self.se_address as u8, &mut data[0..dlen+2]).map_err(|_| T1Error::ReceiveError)?;
		crc = crc16_ccitt_update(crc, &data[0..dlen+2]);
		crc = crc16_ccitt_final(crc);

		if crc != get_u16_le(&data[dlen..dlen+2]) {
			return Err(T1Error::ChecksumError);
		}

		Ok(())
	}
}

impl<TWI> T1Proto for T1overI2C<TWI> where TWI: embedded_hal::blocking::i2c::Read + embedded_hal::blocking::i2c::Write {

	#[inline(never)]
	fn send_apdu(&mut self, apdu: &Apdu) -> Result<(), T1Error> {
		// convert apdu struct into [u8] stream
		// prepend T=1 header (NAD, PCB, LEN)
		// calculate and append T=1 CRC16
		// pass to TWI
		Ok(())
	}

	#[inline(never)]
	fn receive_apdu<'a>(&mut self, buf: &'a mut [u8], apdu: &'a mut Apdu) -> Result<(), T1Error> {
		// receive from TWI into buf (split_at_mut(3)...)
		// parse T=1 header, check PCB (actually APDU?)
			// if found to be S:WTX, directly respond and wait again?
		// check CRC16
		// fill in apdu struct from buf payload
		Ok(())
	}

	#[inline(never)]
	fn interface_soft_reset(&mut self) -> Result<AnswerToReset, T1Error> {
		let mut atrbuf: [u8; 64] = [0u8; 64];
		self.send_s(T1SCode::InterfaceSoftReset, &[])?;
		self.receive_s(T1SCode::InterfaceSoftReset, &mut atrbuf)?;

		let atr_pv = atrbuf[0];
		let dllp_len = atrbuf[6];
		if dllp_len != 4 {
			return Err(T1Error::ProtocolError);
		}
		let plp_type = atrbuf[11];
		let plp_len = atrbuf[12];
		if plp_type != 2 /* I2C */ || plp_len != 11 {
			return Err(T1Error::ProtocolError);
		}
		let hb_len = atrbuf[24];
		Ok(AnswerToReset {
			protocol_version: atr_pv,
			vendor_id: atrbuf[1..6].try_into().unwrap(),
			dllp: DataLinkLayerParameters {
				bwt_ms: get_u16_be(&atrbuf[7..9]),
				ifsc: get_u16_be(&atrbuf[9..11])
			},
			plp: PhysicalLayerParameters::I2C(I2CParameters {
				mcf: get_u16_be(&atrbuf[13..15]),
				configuration: atrbuf[16],
				mpot_ms: atrbuf[17],
				rfu: atrbuf[18..21].try_into().unwrap(),
				segt_us: get_u16_be(&atrbuf[21..23]),
				wut_us: get_u16_be(&atrbuf[23..25])
			}),
			historical_bytes: atrbuf[25..57].try_into().unwrap()
		})
	}
}
