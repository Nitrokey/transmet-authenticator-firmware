use crate::types::*;
use core::convert::{TryFrom, Into};

#[derive(Debug, PartialEq, Eq)]
pub enum Se050Error {
	UnknownError
}

pub const APDU_INSTRUCTION_TRANSIENT: u8 = 0x80;
pub const APDU_INSTRUCTION_AUTH_OBJECT: u8 = 0x40;
pub const APDU_INSTRUCTION_ATTEST: u8 = 0x20;

#[allow(dead_code)]
#[repr(u8)]
pub enum Se050ApduInstruction { /* mask:0x1f */
	Write = 0x01,
	Read = 0x02,
	Crypto = 0x03,
	Mgmt = 0x04,
	Process = 0x05,
	ImportExternal = 0x06
}

#[allow(dead_code)]
#[repr(u8)]
pub enum Se050ApduP1KeyType { /* mask:0x60 */
	KeyPair = 0x60,
	PrivateKey = 0x40,
	PublicKey = 0x20,
}

#[allow(dead_code)]
#[repr(u8)]
pub enum Se050ApduP1CredType {
	Default = 0x00, 
	EC = 0x01,
	RSA = 0x02,
	AES = 0x03,
	DES = 0x04,
	HMAC = 0x05,
	Binary = 0x06,
	UserID = 0x07,
	Counter = 0x08,
	PCR = 0x09,
	Curve = 0x0b,
	Signature = 0x0c,
	MAC = 0x0d, 
	Cipher = 0x0e,
	TLS = 0x0f,
	CryptoObj = 0x10
}       

#[allow(dead_code,non_camel_case_types)]
#[repr(u8)]
pub enum Se050ApduP2 {
	Default = 0x00,
	Generate = 0x03,
	Create = 0x04,
	Size = 0x07,
	Sign = 0x09,
	Verify = 0x0a,
	Init = 0x0b,
	Update = 0x0c,
	Final = 0x0d,
	Oneshot = 0x0e,
	DH = 0x0f,
	Diversify = 0x10,
	AuthFirstPart2 = 0x12,
	AuthNonfirstPart2 = 0x13,
	DumpKey = 0x14,
	ChangeKeyPart1 = 0x15,
	ChangeKeyPart2 = 0x16,
	KillAuth = 0x17, 
	Import = 0x18,
	Export = 0x19,
	SessionCreate = 0x1b,
	SessionClose = 0x1c,
	SessionRefresh = 0x1e,
	SessionPolicy = 0x1f,
	Version = 0x20, 
	Memory = 0x22,
	List = 0x25,
	Type = 0x26,
	Exist = 0x27,
	DeleteObject = 0x28,
	DeleteAll = 0x2a,
	SessionUserID = 0x2c,
	HKDF = 0x2d,
	PBKDF = 0x2e,
	I2CM = 0x30,
	I2CMAttested = 0x31,
	MAC = 0x32,
	UnlockChallenge = 0x33,
	CurveList = 0x34, 
	SignECDAA = 0x35, 
	ID = 0x36,
	EncryptOneshot = 0x37,
	DecryptOneshot = 0x38,
	Attest = 0x3a, 
	Attributes = 0x3b,
	CPLC = 0x3c, 
	Time = 0x3d, 
	Transport = 0x3e,
	Variant = 0x3f,
	Param = 0x40,
	DeleteCurve = 0x41,
	Encrypt = 0x42,
	Decrypt = 0x43,
	Validate = 0x44,
	GenerateOneshot = 0x45,
	ValidateOneshot = 0x46,
	CryptoList = 0x47,
	Random = 0x49,
	TLS_PMS = 0x4a,
	TLS_PRF_CLI_Hello = 0x4b,
	TLS_PRF_SRV_Hello = 0x4c,
	TLS_PRF_CLI_RND = 0x4d,
	TLS_PRF_SRV_RND = 0x4e,
	RAW = 0x4f,
	ImportExt = 0x51,
	SCP = 0x52,
	AuthFirstPart1 = 0x53,
	AuthNonfirstPart1 = 0x54,
}

#[allow(dead_code)]
#[repr(u8)]
pub enum Se050ApduSecObjType {
	ECKeyPair = 0x01,
	ECPrivKey = 0x02,
	ECPubKey = 0x03,
	RSAKeyPair = 0x04,
	RSAKeyPairCRT = 0x05,
	RSAPrivKey = 0x06,
	RSAPrivKeyCRT = 0x07,
	RSAPubKey = 0x08,
	AESKey = 0x09,
	DESKey = 0x0a,
	BinaryFile = 0x0b,
	UserID = 0x0c,
	Counter = 0x0d,
	PCR = 0x0f,
	Curve = 0x10,
	HMACKey = 0x11,
}

#[allow(dead_code)]
#[repr(u8)]
pub enum Se050ApduMemoryType {
	Persistent = 1,
	TransientReset = 2,
	TransientDeselect = 3,
}

#[allow(dead_code)]
#[repr(u8)]
pub enum Se050ApduObjectOrigin {
	External = 1,
	Internal = 2,
	Provisioned = 3
}

#[allow(dead_code)]
#[repr(u8)]
pub enum Se050TlvTag {
	SessionID = 0x10,
	Policy = 0x11,
	MaxAttempts = 0x12,
	ImportAuthData = 0x13,
	ImportAuthKeyID = 0x14,
	Tag1 = 0x41,
	Tag2 = 0x42,
	Tag3 = 0x43,
	Tag4 = 0x44,
	Tag5 = 0x45,
	Tag6 = 0x46,
	Tag7 = 0x47,
	Tag8 = 0x48,
	Tag9 = 0x49,
	Tag10 = 0x4a,
}

include!("se050_convs.rs");

//////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Se050AppInfo {
	applet_version: u32,
	features: u16,
	securebox_version: u16
}

pub struct Se050<T, DP> where T: T1Proto, DP: embedded_hal::blocking::delay::DelayMs<u32> {
	t1_proto: T,
	atr_info: Option<AnswerToReset>,
	app_info: Option<Se050AppInfo>,
	delay_provider: DP,
}

impl<T, DP> Se050<T, DP> where T: T1Proto, DP: embedded_hal::blocking::delay::DelayMs<u32> {

	pub fn new(t1: T, dp: DP) -> Se050<T, DP> {
		Se050 {
			t1_proto: t1,
			atr_info: None,
			app_info: None,
			delay_provider: dp,
		}
	}

	pub fn enable(&mut self) -> Result<(), Se050Error> {
		let r = self.t1_proto.interface_soft_reset();
		if r.is_err() {
			return Err(Se050Error::UnknownError);
		}

		self.atr_info = r.ok();
		debug!("SE050 ATR: {}", self.atr_info.unwrap());
		let app_id: [u8; 16] = [0xA0, 0x00, 0x00, 0x03, 0x96, 0x54, 0x53, 0x00,
                        		0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00];
		let app_select_apdu = CApdu::new(
			ApduClass::StandardPlain,
			ApduStandardInstruction::SelectFile.into(),
			0x04, 0x00, &app_id);
		self.t1_proto.send_apdu(&app_select_apdu, 0).map_err(|_| Se050Error::UnknownError)?;

		let mut appid_data: [u8; 9] = [0; 9];
		let mut appid_apdu = RApdu::blank();

		self.t1_proto.receive_apdu(&mut appid_data, &mut appid_apdu).map_err(|_| Se050Error::UnknownError)?;

		let adata = appid_apdu.data;
		let asw = appid_apdu.sw;
		if asw != 0x9000 || adata.len() != 7 {
			return Err(Se050Error::UnknownError);
		}

		self.app_info = Some(Se050AppInfo {
				applet_version: get_u24_be(&adata[0..3]),
				features: get_u16_be(&adata[3..5]),
				securebox_version: get_u16_be(&adata[5..7])
		});
		debug!("SE050 App: {}", self.app_info.unwrap());

		Ok(())
	}

	pub fn disable(&mut self) {
		// send S:EndApduSession
		// receive ACK
		// power down
	}
}
