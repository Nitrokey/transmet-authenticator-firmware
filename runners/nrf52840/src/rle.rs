use rtt_target;

#[derive(Debug)]
pub enum RLEError {
	HeaderError,
	BufferMismatchError,
	UnsupportedUnitSizeError,
	DecodeError
}

fn u32be_from_slice_off(src: &[u8], off: usize) -> usize {
	let mut val: u32 = 0;

	if off < src.len() {
		val = src[off] as u32;
	}
	if off+1 < src.len() {
		val = (val << 8) + (src[off+1] as u32);
	}
	if off+2 < src.len() {
		val = (val << 8) + (src[off+2] as u32);
	}
	if off+3 < src.len() {
		val = (val << 8) + (src[off+3] as u32);
	}
	val as usize
}

pub fn rle_decode(dest: &mut [u8], src: &[u8]) -> Result<(), RLEError> {
	// check for 'RLE' prefix
	if src[0] != 0x52 || src[1] != 0x4c || src[2] != 0x45 {
		return Err(RLEError::HeaderError);
	}
	let destlen = u32be_from_slice_off(src, 4);
	let mut out_index: usize = 0;
	let mut in_index: usize = 8;
	if dest.len() != destlen {
		return Err(RLEError::BufferMismatchError);
	}
	match src[3] {
	0x48 /* 'H' */ => {
		while (out_index < destlen) && (in_index < src.len()) {
			let prefix = src[in_index];
			let mut count: usize = 0;
			if prefix == 0 {
				if in_index+3 > src.len() { break; }
				count = 1;
				in_index += 1;
			} else if prefix == 1 {
				if in_index+4 > src.len() { break; }
				count = src[in_index+1] as usize;
				in_index += 2;
			} else if prefix == 2 {
				if in_index+5 > src.len() { break; }
				count = ((src[in_index+1] as usize) << 8) + (src[in_index+2] as usize);
				in_index += 3;
				rtt_target::rprintln!("RLE Pfx2");
			}
			for i in 0..count {
				dest[out_index+2*i] = src[in_index];
				dest[out_index+2*i+1] = src[in_index+1];
			}
			in_index += 2;
			out_index += 2*count;
		}
		if (out_index < destlen) || (in_index < src.len()) {
			return Err(RLEError::DecodeError);
		}
		Ok(())
	}
	_ => { return Err(RLEError::UnsupportedUnitSizeError); }
	}
}
