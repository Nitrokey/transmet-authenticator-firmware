// use crate::{rle, TRUSSED_LOGO_RLE};
use embedded_graphics::{
	DrawTarget,
	pixelcolor::{RgbColor, Rgb565, raw::RawU16, raw::RawData},
};
use nrf52840_hal::{
	gpio::{Pin, Input, Output, PullUp, PushPull},
	pac::SPIM0,
	prelude::{InputPin, OutputPin},
	spim::Spim,
};
use trussed::{
	platform::{consent, reboot, ui},
};

type OutPin = Pin<Output<PushPull>>;
type InPin = Pin<Input<PullUp>>;
type LLDisplay = picolcd114::ST7789<display_interface_spi::SPIInterfaceNoCS<Spim<SPIM0>, OutPin>, OutPin>;

enum StickUIState {
	PreInitGarbled,
	Logo,
	Blank,
}

/* sufficient (rgb565) room for a 32x32 sprite or 6 9x18 characters */
static mut DISPLAY_BUF: [u8; 2048] = [0; 2048];
const FONT: &[u8; 62208] = include_bytes!("../data/font_9x18.raw");

pub struct StickUI {
	buf: &'static mut [u8; 2048],
	dsp: Display,
	buttons: [Option<InPin>; 8],
	leds: [Option<OutPin>; 4],
	state: StickUIState,
	last_update: u32
}

impl StickUI {
	pub fn new(dsp: Display, buttons: [Option<InPin>; 8], leds: [Option<OutPin>; 4]) -> Self {
		let xbuf = unsafe { &mut DISPLAY_BUF };
		Self { buf: xbuf, dsp, buttons, leds, state: StickUIState::PreInitGarbled, last_update: 0 }
	}

	pub fn check_buttons(&self) {
		if self.buttons[0].as_ref().map_or_else(|| false, |b| b.is_low().unwrap()) {
			/* do something */
		}
	}

	fn rgb16_memset(&mut self, color: Rgb565) {
		// holy cow, Rust type inference/annotation is so sh*tty...
		let c: u16 = Into::<RawU16>::into(color).into_inner();
		let ch: u8 = (c >> 8) as u8;
		let cl: u8 = (c & 255) as u8;
		let buflen: usize = self.buf.len();

		// the code generated from this looks more complicated than necessary;
		// one day, replace all this nonsense with a tasty call to __aeabi_memset4()
		// or figure out the "proper" Rust incantation the compiler happens to grasp
		for i in (0..buflen).step_by(2) {
			self.buf[i+0] = ch;
			self.buf[i+1] = cl;
		}
	}

	pub fn refresh(&mut self, t: u32) {
		match self.state {
		StickUIState::PreInitGarbled => {
			rtt_target::rprintln!("UI P");
			self.rgb16_memset(embedded_graphics::pixelcolor::Rgb565::BLACK);
			self.tile_bg();
			self.state = StickUIState::Blank;
			self.last_update = t;
			}
		StickUIState::Logo => {
			rtt_target::rprintln!("UI L");
			}
		StickUIState::Blank => {
			rtt_target::rprintln!("UI B");
			self.prepare_text(b"EMCBUF");
			self.dsp.blit_at(self.buf, 240/2-3*9, 135/2-9, 6*9, 18);
			self.state = StickUIState::Logo;
			self.last_update = t;
			}
		}
		if self.leds[0].is_some() {
			match t & 8 {
				0 => { self.leds[0].as_mut().unwrap().set_low().ok(); },
				_ => { self.leds[0].as_mut().unwrap().set_high().ok(); }
			};
		}
	}

	fn prepare_text(&mut self, txt: &[u8]) {
		for i in 0..6 {
			if i >= txt.len() {
				break;
			}
			let mut c: usize = txt[i] as usize;
			if c >= 0x20 && c < 0x80 {		// [0x20:0x7f] map to texture map positions [0x00:0x5f]
				c -= 0x20;
			} else if c >= 0xa0 {			// [0xa0:0xff] map to texture map positions [0x60:0xbf]
				c -= 0x40;
			} else {
				continue;
			}
			rtt_target::rprintln!("Ch {} {}", i, c);

			// memcpy from FONT[c*9*18] to self.buf[bufpos*9*18]
			unsafe { __aeabi_memcpy(self.buf[i*9*18*2] as *mut u8, FONT[c*9*18*2] as *const u8, 9*18*2); }
		}
	}

	fn tile_bg(&mut self) {
		for x in 0..4 {
			for y in 0..9 {
				self.dsp.blit_at(self.buf, x*60, y*15, 60, 15);
			}
		}
	}
}

////////////////////////////////////////////////////////////////////////////////

pub struct WrappedUI {
}

impl WrappedUI {
	pub fn new() -> Self { Self {} }
}

impl trussed::platform::UserInterface for WrappedUI {
	fn check_user_presence(&mut self) -> consent::Level {
		consent::Level::Normal
	}

	fn set_status(&mut self, _status: ui::Status) {
		rtt_target::rprintln!("UI SetStatus");
	}

	fn refresh(&mut self) {}

	fn uptime(&mut self) -> core::time::Duration {
		let _cyccnt = cortex_m::peripheral::DWT::get_cycle_count();
		core::time::Duration::new(0, 0)
	}

	fn reboot(&mut self, to: reboot::To) -> ! {
		match to {
			reboot::To::Application => {
				// set GPREGRET to zero
			}
			reboot::To::ApplicationUpdate => {
				// set GPREGRET to magic value
			}
		}
		// assert soft reset, registers will be retained
		cortex_m::peripheral::SCB::sys_reset();
	}
}

////////////////////////////////////////////////////////////////////////////////

pub struct Display {
	lldisplay: LLDisplay,
	backlight_pin: OutPin,
	power_gate: Option<OutPin>,
}

impl Display {
	/* Maybe we need the CS pin down here as well. The display is the only client
	   on this bus, but maybe GPIO state doesn't persist in deep sleep states, so the
	   pin might require reconfiguration */

	pub fn new(lld: LLDisplay, bl_pin: OutPin, pwr_gate: Option<OutPin>) -> Self {
		Self { lldisplay: lld, backlight_pin: bl_pin, power_gate: pwr_gate }
	}

	pub fn power_off(&mut self) {
		if self.power_gate.is_some() {
			self.power_gate.as_mut().unwrap().set_low().ok();
		}
	}

	pub fn power_on(&mut self) {
		if self.power_gate.is_some() {
			self.power_gate.as_mut().unwrap().set_high().ok();
		}
	}

	pub fn backlight_off(&mut self) {
		self.backlight_pin.set_low().ok();
	}

	pub fn backlight_on(&mut self) {
		self.backlight_pin.set_high().ok();
	}

	pub fn blit_at(&mut self, buf: &[u8], x: u16, y: u16, w: u16, h: u16) {
		self.lldisplay.blit_pixels(x, y, w, h, buf).ok();
	}

	pub fn blit(&mut self, buf: &[u8]) {
		self.blit_at(buf, 0, 0, 240, 135)
	}

	pub fn clear(&mut self, color: Rgb565) {
		self.lldisplay.clear(color).ok();
	}
}

extern "C" {
	fn __aeabi_memcpy(dst: *mut u8, src: *const u8, len: usize);
}
