use nrf52840_hal::{
	clocks::{Clocks, Internal, ExternalOscillator, LfOscStarted},
	usbd::{Usbd, UsbPeripheral},
};
use trussed::{
	Interchange
};
use usb_device::{
	bus::UsbBusAllocator,
	device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
};

type XUsbd<'a> = Usbd<UsbPeripheral<'a>>;
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

macro_rules! bit_event {
($reg:expr, $shift:expr) => {
	match $reg.read().bits() {
	0 => { 0u32 },
	_ => { (1u32 << $shift) }
	}
};
}

pub fn usbd_debug_events() -> u32 {
	let mut v: u32 = 0;
	unsafe {
		let usb_pac = nrf52840_hal::pac::Peripherals::steal().USBD;
		for i in 0..8 {
			v |= bit_event!(usb_pac.events_endepin[i], 2+i);
			v |= bit_event!(usb_pac.events_endepout[i], 12+i);
		}
		v |= bit_event!(usb_pac.events_endisoin, 11);
		v |= bit_event!(usb_pac.events_endisoout, 20);
		v |= bit_event!(usb_pac.events_ep0datadone, 10);
		v |= bit_event!(usb_pac.events_ep0setup, 23);
		v |= bit_event!(usb_pac.events_epdata, 24);
		v |= bit_event!(usb_pac.events_sof, 21);
		v |= bit_event!(usb_pac.events_started, 1);
		v |= bit_event!(usb_pac.events_usbevent, 22);
		v |= bit_event!(usb_pac.events_usbreset, 0);
	}
	v
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
