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
// type LFClockType = Clocks<Internal, Internal, LfOscStarted>;
type LFHFClockType = Clocks<ExternalOscillator, Internal, LfOscStarted>;

pub struct USBPreinitObjects {
	usb_pac: nrf52840_pac::USBD,
	clk: LFHFClockType
}

static mut USBD: Option<UsbBusAllocator<XUsbd>> = None;
static mut USBCLK: Option<LFHFClockType> = None;

pub struct USBObjects<'a> {
	usbdevice: UsbDevice<'a, XUsbd<'a>>,
	ctaphid_class: usbd_ctaphid::CtapHid<'a, XUsbd<'a>>,
}

pub struct USBDispatcher {
	ctaphid_dispatch: ctaphid_dispatch::dispatch::Dispatch,
}

pub fn preinit(usb_pac: nrf52840_pac::USBD, clk: nrf52840_hal::clocks::Clocks<ExternalOscillator, Internal, LfOscStarted>) -> USBPreinitObjects {
	USBPreinitObjects { usb_pac, clk }
}

pub fn init(preinit: USBPreinitObjects) -> (USBObjects<'static>, USBDispatcher) {
	preinit.usb_pac.intenset.write(|w| w
			.usbreset().set_bit()
			.usbevent().set_bit()
			.sof().set_bit()
			.ep0datadone().set_bit()
			.ep0setup().set_bit());

	unsafe { USBCLK.replace(preinit.clk); }
	let usbclk_ref = unsafe { USBCLK.as_ref().unwrap() };

	let usb_peripheral = UsbPeripheral::new(preinit.usb_pac, usbclk_ref);
	unsafe { USBD.replace(Usbd::new(usb_peripheral)); }
	let usbd_ref = unsafe { USBD.as_ref().unwrap() };

	rtt_target::rprintln!("USB: Glbl ok");

	let (ctaphid_rq, ctaphid_rp) = ctaphid_dispatch::types::HidInterchange::claim().unwrap();
	let ctaphid = usbd_ctaphid::CtapHid::new(usbd_ref, ctaphid_rq, 0u32)
			.implements_ctap1()
			.implements_ctap2()
			.implements_wink();
	let ctaphid_dispatch = ctaphid_dispatch::dispatch::Dispatch::new(ctaphid_rp);
	let usbdevice = UsbDeviceBuilder::new(usbd_ref, UsbVidPid(0x1209, 0x5090))
			.product("EMC Stick").manufacturer("Nitrokey/PTB")
			.serial_number("imagine-a-uuid-here")
			.device_release(0x0001u16)
			.max_packet_size_0(64).build();

	rtt_target::rprintln!("USB: Objx ok");

	( USBObjects { usbdevice, ctaphid_class: ctaphid },
		USBDispatcher { ctaphid_dispatch } )
}

impl USBObjects<'static> {
	// Polls for activity from the host (called from the USB IRQ handler) //
	pub fn poll(&mut self) {
		self.ctaphid_class.check_for_app_response();
		self.usbdevice.poll(&mut [&mut self.ctaphid_class]);
	}
}

impl USBDispatcher {
	// Polls for activity from the userspace applications (called during IDLE) //
	pub fn poll_apps(&mut self, apps: &mut [&mut dyn ctaphid_dispatch::app::App]) -> bool {
		self.ctaphid_dispatch.poll(apps)
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
