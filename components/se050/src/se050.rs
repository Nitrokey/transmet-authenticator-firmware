use crate::types::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Se050Error {
}

pub struct Se050<T, DP> where T: crate::t1::T1Proto, DP: embedded_hal::blocking::delay::DelayMs<u32> {
	t1_proto: T,
	// power_pin: Pin<Output<PushPull>>,
	atr_info: Option<AnswerToReset>,
	delay_provider: DP,
}

impl<T, DP> Se050<T, DP> where T: crate::t1::T1Proto, DP: embedded_hal::blocking::delay::DelayMs<u32> {

	pub fn new(t1: T, dp: DP) -> Se050<T, DP> {
		Se050 {
			t1_proto: t1,
			atr_info: None,
			delay_provider: dp,
		}
	}
}
