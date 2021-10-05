use core::convert::TryFrom;

pub enum Iso7816Error {
	ValueError
}

#[allow(dead_code)]
#[repr(u8)]
pub enum ApduClass {
	Standard = 0b1000_0000,
	ProprietarySecure = 0b1000_0100,
}

pub const APDU_INSTRUCTION_TRANSIENT: u8 = 0x80;
pub const APDU_INSTRUCTION_AUTH_OBJECT: u8 = 0x40;
pub const APDU_INSTRUCTION_ATTEST: u8 = 0x20;

#[allow(dead_code)]
#[repr(u8)]
pub enum ApduInstruction {
	Write = 0x01,
	Read = 0x02,
	Crypto = 0x03,
	Mgmt = 0x04,
	Process = 0x05,
	ImportExternal = 0x06
}

impl TryFrom<u8> for ApduInstruction {
	type Error = Iso7816Error;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value & 0x1f_u8 {
		0x01 => Ok(Self::Write),
		0x02 => Ok(Self::Read),
		0x03 => Ok(Self::Crypto),
		0x04 => Ok(Self::Mgmt),
		0x05 => Ok(Self::Process),
		0x06 => Ok(Self::ImportExternal),
		_ => Err(Self::Error::ValueError)
		}
	}
}

#[allow(dead_code)]
#[repr(u8)]
pub enum ApduP1KeyType {
	KeyPair = 0x60,
	PrivateKey = 0x40,
	PublicKey = 0x20,
}

impl TryFrom<u8> for ApduP1KeyType {
	type Error = Iso7816Error;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value & 0x60_u8 {
		0x20 => Ok(Self::KeyPair),
		0x40 => Ok(Self::PrivateKey),
		0x60 => Ok(Self::PublicKey),
		_ => Err(Self::Error::ValueError)
		}
	}
}

#[allow(dead_code)]
#[repr(u8)]
pub enum ApduP1CredType {
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
pub enum ApduP2 {
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
pub enum ApduSecObjType {
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
pub enum ApduMemoryType {
	Persistent = 1,
	TransientReset = 2,
	TransientDeselect = 3,
}

#[allow(dead_code)]
#[repr(u8)]
pub enum ApduObjectOrigin {
	External = 1,
	Internal = 2,
	Provisioned = 3
}

#[allow(dead_code)]
#[repr(u8)]
pub enum TlvTag {
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

//////////////////////////////////////////////////////////////////////////////

pub struct Tlv<'a> {
	tag: TlvTag,
	data: &'a [u8]
}

pub struct Apdu<'a> {
	cla: ApduClass,
	ins_char_transient: bool,
	ins_char_authobject: bool,
	ins_char_attest: bool,
	ins: ApduInstruction,
	p1k: ApduP1KeyType,
	p1c: ApduP1CredType,
	p2: ApduP2,
	tags: &'a [Tlv<'a>]
}

//////////////////////////////////////////////////////////////////////////////

pub const T1_S_REQUEST_CODE: u8 = 0b1100_0000;
pub const T1_S_RESPONSE_CODE: u8 = 0b1110_0000;

pub const T1_R_CODE_MASK: u8 = 0b1110_1100;
pub const T1_R_CODE: u8 = 0b1000_0000;

pub enum T1SCode {
	Resync = 0,
	IFS = 1,
	Abort = 2,
	WTX = 3,
	EndApduSession = 5,
	ChipReset = 6,
	GetATR = 7,
	InterfaceSoftReset = 15
}

impl TryFrom<u8> for T1SCode {
	type Error = Iso7816Error;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::Resync),
			1 => Ok(Self::IFS),
			2 => Ok(Self::Abort),
			3 => Ok(Self::WTX),
			5 => Ok(Self::EndApduSession),
			6 => Ok(Self::ChipReset),
			7 => Ok(Self::GetATR),
			15 => Ok(Self::InterfaceSoftReset),
			_ => Err(Self::Error::ValueError)
		}
	}
}

impl Into<u8> for T1SCode {
	fn into(self) -> u8 {
		match self {
		Self::Resync => 0,
		Self::IFS => 1,
		Self::Abort => 2,
		Self::WTX => 3,
		Self::EndApduSession => 5,
		Self::ChipReset => 6,
		Self::GetATR => 7,
		Self::InterfaceSoftReset => 15,
		}
	}
}

pub enum T1Error {
	TransmitError,
	ReceiveError,
	BufferOverrunError(usize),
	ChecksumError,
	ProtocolError,
	RCodeReceived(u8),
}

//////////////////////////////////////////////////////////////////////////////

pub struct AnswerToReset {
	pub protocol_version: u8,
	pub vendor_id: [u8; 5],
	// Data Link Layer Parameters
	pub dllp: DataLinkLayerParameters,
	// Physical Layer Parameters
	pub plp: PhysicalLayerParameters,
	// Historical Bytes (truncated to save memory)
	pub historical_bytes: [u8; 32]
}

pub struct DataLinkLayerParameters {
	pub bwt_ms: u16,
	pub ifsc: u16,
}

pub enum PhysicalLayerParameters {
	I2C(I2CParameters)
}

pub struct I2CParameters {
	pub mcf: u16,
	pub configuration: u8,
	pub mpot_ms: u8,
	pub rfu: [u8; 3],
	pub segt_us: u16,
	pub wut_us: u16,
}

//////////////////////////////////////////////////////////////////////////////

const CRC16_CCITT_XORLUT: [u16; 256] = [
	0x0000, 0x1189, 0x2312, 0x329b, 0x4624, 0x57ad, 0x6536, 0x74bf,
	0x8c48, 0x9dc1, 0xaf5a, 0xbed3, 0xca6c, 0xdbe5, 0xe97e, 0xf8f7,
	0x1081, 0x0108, 0x3393, 0x221a, 0x56a5, 0x472c, 0x75b7, 0x643e,
	0x9cc9, 0x8d40, 0xbfdb, 0xae52, 0xdaed, 0xcb64, 0xf9ff, 0xe876,
	0x2102, 0x308b, 0x0210, 0x1399, 0x6726, 0x76af, 0x4434, 0x55bd,
	0xad4a, 0xbcc3, 0x8e58, 0x9fd1, 0xeb6e, 0xfae7, 0xc87c, 0xd9f5,
	0x3183, 0x200a, 0x1291, 0x0318, 0x77a7, 0x662e, 0x54b5, 0x453c,
	0xbdcb, 0xac42, 0x9ed9, 0x8f50, 0xfbef, 0xea66, 0xd8fd, 0xc974,
	0x4204, 0x538d, 0x6116, 0x709f, 0x0420, 0x15a9, 0x2732, 0x36bb,
	0xce4c, 0xdfc5, 0xed5e, 0xfcd7, 0x8868, 0x99e1, 0xab7a, 0xbaf3,
	0x5285, 0x430c, 0x7197, 0x601e, 0x14a1, 0x0528, 0x37b3, 0x263a,
	0xdecd, 0xcf44, 0xfddf, 0xec56, 0x98e9, 0x8960, 0xbbfb, 0xaa72,
	0x6306, 0x728f, 0x4014, 0x519d, 0x2522, 0x34ab, 0x0630, 0x17b9,
	0xef4e, 0xfec7, 0xcc5c, 0xddd5, 0xa96a, 0xb8e3, 0x8a78, 0x9bf1,
	0x7387, 0x620e, 0x5095, 0x411c, 0x35a3, 0x242a, 0x16b1, 0x0738,
	0xffcf, 0xee46, 0xdcdd, 0xcd54, 0xb9eb, 0xa862, 0x9af9, 0x8b70,
	0x8408, 0x9581, 0xa71a, 0xb693, 0xc22c, 0xd3a5, 0xe13e, 0xf0b7,
	0x0840, 0x19c9, 0x2b52, 0x3adb, 0x4e64, 0x5fed, 0x6d76, 0x7cff,
	0x9489, 0x8500, 0xb79b, 0xa612, 0xd2ad, 0xc324, 0xf1bf, 0xe036,
	0x18c1, 0x0948, 0x3bd3, 0x2a5a, 0x5ee5, 0x4f6c, 0x7df7, 0x6c7e,
	0xa50a, 0xb483, 0x8618, 0x9791, 0xe32e, 0xf2a7, 0xc03c, 0xd1b5,
	0x2942, 0x38cb, 0x0a50, 0x1bd9, 0x6f66, 0x7eef, 0x4c74, 0x5dfd,
	0xb58b, 0xa402, 0x9699, 0x8710, 0xf3af, 0xe226, 0xd0bd, 0xc134,
	0x39c3, 0x284a, 0x1ad1, 0x0b58, 0x7fe7, 0x6e6e, 0x5cf5, 0x4d7c,
	0xc60c, 0xd785, 0xe51e, 0xf497, 0x8028, 0x91a1, 0xa33a, 0xb2b3,
	0x4a44, 0x5bcd, 0x6956, 0x78df, 0x0c60, 0x1de9, 0x2f72, 0x3efb,
	0xd68d, 0xc704, 0xf59f, 0xe416, 0x90a9, 0x8120, 0xb3bb, 0xa232,
	0x5ac5, 0x4b4c, 0x79d7, 0x685e, 0x1ce1, 0x0d68, 0x3ff3, 0x2e7a,
	0xe70e, 0xf687, 0xc41c, 0xd595, 0xa12a, 0xb0a3, 0x8238, 0x93b1,
	0x6b46, 0x7acf, 0x4854, 0x59dd, 0x2d62, 0x3ceb, 0x0e70, 0x1ff9,
	0xf78f, 0xe606, 0xd49d, 0xc514, 0xb1ab, 0xa022, 0x92b9, 0x8330,
	0x7bc7, 0x6a4e, 0x58d5, 0x495c, 0x3de3, 0x2c6a, 0x1ef1, 0x0f78,
];

pub fn crc16_ccitt_oneshot(buf: &[u8]) -> u16 {
	let mut crc: u16 = crc16_ccitt_init();
	crc = crc16_ccitt_update(crc, buf);
	crc16_ccitt_final(crc)
}

pub fn crc16_ccitt_init() -> u16 { 0xffff }

pub fn crc16_ccitt_update(mut crc: u16, buf: &[u8]) -> u16 {
	for i in 0..buf.len() {
		let lutbyte: u8 = (crc ^ (buf[i] as u16)) as u8;
		crc = (crc >> 8) ^ CRC16_CCITT_XORLUT[lutbyte as usize];
	}
	crc
}

pub fn crc16_ccitt_final(crc: u16) -> u16 { crc ^ 0xffff }

pub fn get_u16_le(buf: &[u8]) -> u16 {
	(buf[0] as u16) | ((buf[1] as u16) << 8)
}

pub fn set_u16_le(buf: &mut [u8], crc: u16) {
	buf[0] = crc as u8;
	buf[1] = (crc >> 8) as u8;
}

pub fn get_u16_be(buf: &[u8]) -> u16 {
	(buf[1] as u16) | ((buf[0] as u16) << 8)
}

pub fn set_u16_be(buf: &mut [u8], crc: u16) {
	buf[1] = crc as u8;
	buf[0] = (crc >> 8) as u8;
}
