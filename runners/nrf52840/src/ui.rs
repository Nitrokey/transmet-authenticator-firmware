use crate::{rle, TRUSSED_LOGO_RLE};
use embedded_graphics::{
	DrawTarget,
	pixelcolor::{RgbColor, Rgb565, raw::RawU16, raw::RawData},
};
use nrf52840_hal::{
	clocks::{Clocks, Internal, ExternalOscillator, LfOscStarted},
	gpio::{Pin, Input, Output, PullUp, PushPull},
	pac::SPIM0,
	prelude::{InputPin, OutputPin},
	spim::Spim,
	usbd::{Usbd, UsbPeripheral},
};
use trussed::{
	platform::{consent, reboot, ui},
	Interchange
};
use usb_device::{
	bus::UsbBusAllocator,
	device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
};

type OutPin = Pin<Output<PushPull>>;
type InPin = Pin<Input<PullUp>>;
type LLDisplay = picolcd114::ST7789<display_interface_spi::SPIInterfaceNoCS<Spim<SPIM0>, OutPin>, OutPin>;

type XUsbd<'a> = Usbd<UsbPeripheral<'a>>;

enum StickUIState {
	PreInitGarbled,
	Logo,
	Blank,
	
}

static mut DISPLAY_BUF: [u8; 64800] = [0; 64800];

pub struct StickUI {
	buf: &'static mut [u8; 64800],
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
		}
	}

	fn rgb16_memset(buf: &mut [u8], color: Rgb565) {
		// holy cow, Rust type inference/annotation is so sh*tty...
		let c: u16 = Into::<RawU16>::into(color).into_inner();
		let ch: u8 = (c >> 8) as u8;
		let cl: u8 = (c & 255) as u8;
		let mut i: usize = 0;
		let buflen: usize = buf.len();

		// the code generated from this is super-crappy and contains lots of
		// panic_bounds_check() even though it should be trivial to prove
		// that no violation can take place
		// one day, replace all this nonsense with a tasty call to __aeabi_memset4()
		// or figure out the "proper" Rust incantation the compiler happens to grasp
		// PS: somebody know a way to iterate over every _other_ element without
		// happy iterator complexity from the 'std' crate? I don't.
		while i+3 < buflen {
			buf[i+0] = ch;
			buf[i+1] = cl;
			buf[i+2] = ch;
			buf[i+3] = cl;
			i += 4;
		}
	}

	pub fn refresh(&mut self, t: u32) {
		match self.state {
		StickUIState::PreInitGarbled => {
			let logo_decode = rle::rle_decode(self.buf, TRUSSED_LOGO_RLE);
			if logo_decode.is_ok() {
				self.dsp.blit(self.buf);
				self.state = StickUIState::Logo;
			} else {
				StickUI::rgb16_memset(self.buf, embedded_graphics::pixelcolor::Rgb565::BLACK);
				self.dsp.blit(self.buf);
				self.state = StickUIState::Blank;
			}
			self.last_update = t;
			}
		StickUIState::Logo => {
				if self.last_update + 32 < t {
					StickUI::rgb16_memset(self.buf, embedded_graphics::pixelcolor::Rgb565::BLACK);
					self.dsp.blit(self.buf);
					self.state = StickUIState::Blank;
					self.last_update = t;
				}
			}
		StickUIState::Blank => {
			}
		}
		match t & 8 {
			0 => self.leds[0].as_mut().and_then(|l| Some(l.set_low())),
			_ => self.leds[0].as_mut().and_then(|l| Some(l.set_high()))
		};
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
		consent::Level::None
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

	pub fn blit(&mut self, buf: &[u8]) {
		self.lldisplay.blit_pixels(0, 0, 240, 135, buf).ok();
	}

	pub fn clear(&mut self, color: Rgb565) {
		self.lldisplay.clear(color).ok();
	}
}

////////////////////////////////////////////////////////////////////////////////

type LFClockType = Clocks<Internal, Internal, LfOscStarted>;
type LFHFClockType = Clocks<ExternalOscillator, Internal, LfOscStarted>;

static mut LFCLOCK: Option<LFClockType> = None;
static mut LFHFCLOCK: Option<LFHFClockType> = None;

pub enum USBControllerEnum {
	Fake,
	Real(USBController<'static>)
}

pub struct USBController<'a> {
	usbd: UsbBusAllocator<XUsbd<'a>>,
	usbdevice: Option<UsbDevice<'a, XUsbd<'a>>>,
	ctaphid_class: Option<usbd_ctaphid::CtapHid<'a, XUsbd<'a>>>,
	ctaphid_dispatch: Option<ctaphid_dispatch::dispatch::Dispatch>
}

impl<'a> USBController<'a> {
	pub fn new() -> Self { unsafe {
		LFHFCLOCK = Some(LFCLOCK.take().unwrap().enable_ext_hfosc());
		let usb_pac = nrf52840_hal::pac::Peripherals::steal().USBD;
		usb_pac.intenset.write(|w| w.usbreset().set_bit().usbevent().set_bit().sof().set_bit().ep0datadone().set_bit().ep0setup().set_bit());
		let usb_peripheral = UsbPeripheral::new(usb_pac, LFHFCLOCK.as_ref().unwrap());
		rtt_target::rprintln!("USBper");
		Self {
			usbd: Usbd::new(usb_peripheral),
			usbdevice: None,
			ctaphid_class: None,
			ctaphid_dispatch: None
		}
	}}

	pub fn activate(&'a mut self) {
		let (ctaphid_rq, ctaphid_rp) = ctaphid_dispatch::types::HidInterchange::claim().unwrap();
		let ctaphid = usbd_ctaphid::CtapHid::new(&self.usbd, ctaphid_rq, 0u32)
				.implements_ctap1()
				.implements_ctap2()
				.implements_wink();
		self.ctaphid_class = Some(ctaphid);
		self.ctaphid_dispatch = Some(ctaphid_dispatch::dispatch::Dispatch::new(ctaphid_rp));
		self.usbdevice = Some(
				UsbDeviceBuilder::new(&self.usbd, UsbVidPid(0x1209, 0x5090))
				.product("EMC Stick").manufacturer("Nitrokey/PTB")
				.serial_number("imagine-a-uuid-here")
				.device_release(0x0001u16)
				.max_packet_size_0(64).build());
		rtt_target::rprintln!("USBdev");
	}

	/* Polls for activity from the host (called from the USB IRQ handler) */
	pub fn poll(&mut self) {
		let usbdev: &mut UsbDevice<XUsbd> = self.usbdevice.as_mut().unwrap();
		let ctaphid: &mut usbd_ctaphid::CtapHid<XUsbd> = self.ctaphid_class.as_mut().unwrap();

		ctaphid.check_for_app_response();
		usbdev.poll(&mut [ctaphid]);
	}

	/* Polls for activity from the userspace applications (called during IDLE) */
	pub fn poll_apps(&mut self, apps: &mut [&mut dyn ctaphid_dispatch::app::App]) -> bool {
		self.ctaphid_dispatch.as_mut().unwrap().poll(apps)
	}
}

static mut USBCTL: USBControllerEnum = USBControllerEnum::Fake;

/* ZST so we can carry it around as a resource in RTIC */
pub struct USBControllerProxy { }
impl USBControllerProxy {
	pub fn new(clk: LFClockType) -> Self {
		unsafe { LFCLOCK = Some(clk); }
		Self {}
	}

	pub fn access(&self) -> &'static mut USBControllerEnum { unsafe { &mut USBCTL } }

	pub fn instantiate(&self) {
		let usbctlenum = self.access();
		match usbctlenum {
		USBControllerEnum::Real(_) => { },
		USBControllerEnum::Fake => {
			rtt_target::rprintln!("USBf>r");
			unsafe { USBCTL = USBControllerEnum::Real(USBController::new());
			if let USBControllerEnum::Real(r2) = &mut USBCTL {
				r2.activate();
			}}
		}}
	}

	pub fn deinstantiate(&self) {
	}
}
