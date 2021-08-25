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
	Idle,
	// ShowRequest
}

pub enum StickBatteryState {
	Unknown,
	Discharging(u32),
	Charging(u32),		/* supposedly via USB */
}

/* sufficient (rgb565) room for a 32x32 sprite, a 60x15 sprite or six 9x18 characters */
static mut DISPLAY_BUF: [u8; 2048] = [0; 2048];
const FONT: &[u8; (9*18*2)*192] = include_bytes!("../data/font_9x18.raw");
const BATTERY: &[u8; (25*60*2)] = include_bytes!("../data/texmap.raw");

pub struct StickUI {
	buf: &'static mut [u8; 2048],
	dsp: Display,
	buttons: [Option<InPin>; 8],
	leds: [Option<OutPin>; 4],
	state: StickUIState,
	update_due: u32,
	battery_state: StickBatteryState,
}

impl StickUI {
	pub fn new(dsp: Display, buttons: [Option<InPin>; 8], leds: [Option<OutPin>; 4]) -> Self {
		let xbuf = unsafe { &mut DISPLAY_BUF };
		Self {
			buf: xbuf, dsp, buttons, leds,
			state: StickUIState::PreInitGarbled,
			battery_state: StickBatteryState::Unknown,
			update_due: 0 }
	}

	pub fn check_buttons(&self) {
		for i in 0..8 {
			if self.buttons[i].as_ref().map_or_else(|| false, |b| b.is_low().unwrap()) {
				rtt_target::rprintln!("Button {}", i);
			}
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
			self.buf[i+0] = cl;
			self.buf[i+1] = ch;
		}
	}

	pub fn refresh(&mut self, t: u32) {
		/* LED heartbeat (DK only, our prototype has no LEDs) */
		if self.leds[0].is_some() {
			if (t & 15) == 0 {
				self.leds[0].as_mut().unwrap().set_low().ok();
			} else if (t & 15) == 8 {
				self.leds[0].as_mut().unwrap().set_high().ok();
			}
		}

		if t < self.update_due {
			return;
		}

		match self.state {
		StickUIState::PreInitGarbled => {
			rtt_target::rprintln!("UI P");
			self.rgb16_memset(embedded_graphics::pixelcolor::Rgb565::BLACK);
			self.tile_bg();
			self.state = StickUIState::Logo;
			self.update_due = t+1;
			}
		StickUIState::Logo => {
			/* blit some fancy logo once we have one ... */
			self.render_text(b"-LOGO-", 10, 3);
			self.state = StickUIState::Idle;
			self.update_due = t + 32;
			}
		StickUIState::Idle => {
			rtt_target::rprintln!("UI B");
			// self.rgb16_memset(embedded_graphics::pixelcolor::Rgb565::BLACK);
			// self.tile_bg();
			let battsprite: isize = match self.battery_state {
			StickBatteryState::Unknown => { 1 },
			StickBatteryState::Charging(x) => { charge_ani_frame(t, x) },
			StickBatteryState::Discharging(x) => { charge_ani_frame(0, x) }
			};
			unsafe { __aeabi_memcpy(self.buf as *mut u8, (BATTERY as *const u8).offset(battsprite*25*10*2), 25*10*2); }
			self.dsp.blit_at(&self.buf[0..25*10*2], 240-26, 2, 25, 10);
			self.update_due = t + 8;
			}
		}
	}

	pub fn set_battery(&mut self, bat: StickBatteryState) {
		self.battery_state = bat;
	}

	/*
	 * Render a line of text at a given starting _text_ position.
	 * Current screen real estate plan: 26x7 characters of a 9x18 font,
	 * with an extra vertical space pixel (so effectively on a 9x19 grid).
	 * Borders: top 1, left 1, right 5, bottom 1.
	 * Pixel positions of top left glyph (cx,cy == 0,0): [1,1]--[9,19]
	 * Pixel positions of bottom right glyph (cx,cy == 25,6): [226,115]--[234,133]
	 */
	fn render_text(&mut self, txt: &[u8], cx: u16, cy: u16) {
		use core::cmp::min;

		let mut cx_: u16 = cx;
		let mut txtoff: u16 = 0;
		let txtlen: u16 = txt.len() as u16;

		while cx_ < 26 && txtoff < txtlen {
			let chklen = min(min(26 - cx_, txtlen - txtoff), 6);
			self.prepare_text(&txt[txtoff as usize..(txtoff+chklen) as usize]);
			self.dsp.blit_at(&self.buf[0..(chklen*9*18*2) as usize], (cx_*9)+1, (cy*19)+1, chklen*9, 18);
			cx_ += chklen;
			txtoff += chklen;
		}
	}

	fn prepare_text(&mut self, txt: &[u8]) {
		use core::convert::TryInto;

		let mut txtlen = txt.len();
		if txt[txtlen-1] == 0 {		/* chop off trailing null byte */
			txtlen -= 1;
		}
		if txtlen > 6 {
			self.prepare_text(b"LENERR");
			return;
		}
		for i in 0..6 {
			if i >= txt.len() {
				break;
			}
			let c: usize =
			if txt[i] >= 0x20 && txt[i] < 0x80 {		// [0x20:0x7f] map to font positions [0x00:0x5f]
				(txt[i] - 0x20) as usize
			} else if txt[i] >= 0xa0 {			// [0xa0:0xff] map to font positions [0x60:0xbf]
				(txt[i] - 0x40) as usize
			} else {					// illegal char ('admit one' symbol)
				(0x7f - 0x20) as usize
			};
			// rtt_target::rprintln!("Ch {} {}", i, c);
			for y in 0..18 {
				let doff: isize = ((y*txtlen + i)*9*2).try_into().unwrap();
				let soff: isize = ((c*18 + y)*9*2).try_into().unwrap();
				unsafe { __aeabi_memcpy((self.buf as *mut u8).offset(doff),
						(FONT as *const u8).offset(soff),
						9*2); }
			}
		}
	}

	fn tile_bg(&mut self) {
		for x in 0..4 {
			for y in 0..9 {
				self.dsp.blit_at(&self.buf[0..60*15*2], x*60, y*15, 60, 15);
			}
		}
	}
}

fn charge_ani_frame(t: u32, perc: u32) -> isize {
	let aniframe = (t >> 3) & 7;
	if aniframe >= 4 {
		return 0;
	}
	let aniperc = perc + ((100-perc) * aniframe) / 4;
	if aniperc <= 15 { return 2; }
	else if aniperc <= 50 { return 3; }
	else if aniperc <= 80 { return 4; }
	else { return 5; }
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
		// use core::iter::repeat;
		// let constcol = Into::<RawU16>::into(embedded_graphics::pixelcolor::Rgb565::YELLOW).into_inner();

		let r = self.lldisplay.blit_pixels(x, y, w, h, buf);
		if r.is_err() {
			rtt_target::rprintln!("BlitAt ERR");
		}
		// self.lldisplay.set_pixels(x, y, x+w-1, y+h-1, repeat(constcol)).ok();
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
