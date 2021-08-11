#![no_std]
#![no_main]

use panic_halt as _;
// use cortex_m;
use asm_delay::bitrate::U32BitrateExt;
use generic_array::typenum::consts;
use littlefs2::const_ram_storage;
use nrf52840_hal::{
	clocks::Clocks,
	gpiote::Gpiote,
	rng::Rng,
	rtc::{Rtc, RtcInterrupt},
	spim::Spim,
	uarte::{Baudrate, Parity, Uarte},
};
use rand_core::SeedableRng;
use trussed::{
	Interchange,
	types::{LfsResult, LfsStorage},
};

#[cfg(feature = "board-nrfdk")]
use crate::board_nrfdk as board;
#[cfg(feature = "board-proto1")]
use crate::board_proto1 as board;

#[cfg(not(any(feature = "board-nrfdk", feature = "board-proto1")))]
compile_error!{"No board target chosen! Set your board using --feature; see Cargo.toml."}

#[cfg(feature = "board-nrfdk")]
mod board_nrfdk;
#[cfg(feature = "board-proto1")]
mod board_proto1;
mod rle;
mod types;
mod ui;

static TRUSSED_LOGO_RLE: &[u8; 4582] = include_bytes!("../trussed_logo.img.rle");

/* Temporary Hack #2: a Trussed Store that exists solely in RAM (no persistence!) */
littlefs2::const_ram_storage!(InternalStore, 1024);
littlefs2::const_ram_storage!(ExternalStore, 1024);
littlefs2::const_ram_storage!(VolatileStore, 1024);
trussed::store!(
	StickStore,
	Internal: InternalStore,
	External: ExternalStore,
	Volatile: VolatileStore
);

/* Combining our temporary hacks... */
trussed::platform!(
	StickPlatform,
	R: chacha20::ChaCha8Rng,
	S: StickStore,
	UI: ui::WrappedUI,
);

pub struct NRFSyscall {}
impl trussed::platform::Syscall for NRFSyscall {
	fn syscall(&mut self) {
		rtic::pend(nrf52840_hal::pac::Interrupt::SWI0_EGU0);
	}
}

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
	struct Resources {
		gpiote: Gpiote,
		ui: ui::StickUI,
		trussed_service: trussed::service::Service<StickPlatform>,
		uart: Uarte<nrf52840_hal::pac::UARTE0>,
		usbctl_proxy: ui::USBControllerProxy,
		// usbctl: ui::USBControllerEnum,
		// clock: Clocks<ExternalOscillator, nrf52840_hal::clocks::Internal, LfOscStarted>,
		// usbd: UsbDevice<'static, Usbd<UsbPeripheral<'static>>>,
		// hidclass: HIDClass<'static, Usbd<UsbPeripheral<'static>>>,
		power: nrf52840_hal::pac::POWER,
		rtc: Rtc<nrf52840_hal::pac::RTC0>,
		fido_app: dispatch_fido::Fido<fido_authenticator::NonSilentAuthenticator, trussed::ClientImplementation<NRFSyscall>>,
	}

	#[init(spawn = [frontend])]
	fn init(mut ctx: init::Context) -> init::LateResources {
		ctx.core.DCB.enable_trace();
		ctx.core.DWT.enable_cycle_counter();

		rtt_target::rtt_init_print!();

		board::init_early(&ctx.device, &ctx.core);

		rtt_target::rprintln!("Peripheral Wrappers");

		let gpiote = Gpiote::new(ctx.device.GPIOTE);
		let p0 = nrf52840_hal::gpio::p0::Parts::new(ctx.device.P0);
		let p1 = nrf52840_hal::gpio::p1::Parts::new(ctx.device.P1);
		let rng = Rng::new(ctx.device.RNG);
		let power = ctx.device.POWER;
		let mut rtc = Rtc::new(ctx.device.RTC0, 4095).unwrap();

		/* RTIC actively hides cortex_m::peripherals::SYST from us, so we cannot use
		nrf52840_hal::delay - hack around it by using a plain old
		"assembly delay loop" based on the (hardcoded) CPU frequency */
		let mut delay_provider = asm_delay::AsmDelay::new(64_u32.mhz());

		rtt_target::rprintln!("Pins");

		let mut board_gpio = board::init_gpio(&gpiote, p0, p1);

		rtt_target::rprintln!("UART");

		let uart_pins = nrf52840_hal::uarte::Pins {
					txd: board_gpio.uart_tx.take().unwrap(),
					rxd: board_gpio.uart_rx.take().unwrap(),
					cts: board_gpio.uart_cts,
					rts: board_gpio.uart_rts
		};
		let uart = Uarte::new(ctx.device.UARTE0, uart_pins,
				Parity::EXCLUDED, Baudrate::BAUD115200,
		);

		rtt_target::rprintln!("Display");

		let spi = Spim::new(ctx.device.SPIM0, board_gpio.display_spi.take().unwrap(),
			nrf52840_hal::spim::Frequency::M8,
			nrf52840_hal::spim::MODE_3,
			0x7e_u8,
		);
		let di_spi = display_interface_spi::SPIInterfaceNoCS::new(spi, board_gpio.display_dc.take().unwrap());
		let mut dsp_st7789 = picolcd114::ST7789::new(di_spi, board_gpio.display_reset.take().unwrap(), 240, 135, 40, 53);

		dsp_st7789.init(&mut delay_provider).unwrap();

		let disp = ui::Display::new(dsp_st7789,
				board_gpio.display_backlight.take().unwrap(),
				board_gpio.display_power.take());
		let ui = ui::StickUI::new(disp, board_gpio.buttons, board_gpio.leds);

		/* WIP: put together our hacked up LEGO bricks to create the Trussed service instance */
		rtt_target::rprintln!("Trussed Components");

		let stickstore = StickStore::attach_else_format(
			InternalStore::new(),
			ExternalStore::new(),
			VolatileStore::new(),
		);

		let stickplat = StickPlatform::new(
			chacha20::ChaCha8Rng::from_rng(rng).unwrap(),
			stickstore,
			ui::WrappedUI::new()
		);

		let mut srv = trussed::service::Service::new(stickplat);

		rtt_target::rprintln!("Apps");

		let fido_trussed_xch = trussed::pipe::TrussedInterchange::claim().unwrap();
		let mut fido_lfs2_path = littlefs2::path::PathBuf::new();
		fido_lfs2_path.push(littlefs2::path::Path::from_bytes_with_nul(b"fido\0").unwrap());
		srv.add_endpoint(fido_trussed_xch.1, fido_lfs2_path).ok();
		let fido_trussed_client = trussed::ClientImplementation::<NRFSyscall>::new(fido_trussed_xch.0, NRFSyscall {});
		let fido_auth = fido_authenticator::Authenticator::new(fido_trussed_client, fido_authenticator::NonSilentAuthenticator {});
		let fido_app = dispatch_fido::Fido::<fido_authenticator::NonSilentAuthenticator, trussed::ClientImplementation<NRFSyscall>>::new(fido_auth);

		rtt_target::rprintln!("USB");

		let usbctl_proxy = ui::USBControllerProxy::new(Clocks::new(ctx.device.CLOCK).start_lfclk());

		rtt_target::rprintln!("Finalizing");

		// RTIC enables the interrupt during init if there is a handler function bound to it
		rtc.enable_interrupt(RtcInterrupt::Tick, None);
		rtc.enable_counter();

		ctx.spawn.frontend().ok();

		gpiote.port().enable_interrupt();
		power.intenset.write(|w| w.pofwarn().set_bit().usbdetected().set_bit().usbremoved().set_bit().usbpwrrdy().set_bit());

		init::LateResources {
			gpiote,
			ui,
			trussed_service: srv,
			uart,
			// clock: clock2,
			// usbd: usb_dev,
			// hidclass: hid_desc,
			// usbctl,
			usbctl_proxy,
			power,
			rtc,
			fido_app
		}
	}

	#[idle()]
	fn idle(_ctx: idle::Context) -> ! {
		/*
		   Note: ARM SysTick stops in WFI. This is unfortunate as
		   - RTIC uses SysTick for its schedule() feature
		   - we would really like to use WFI in order to save power
		   In the future, we might even consider entering "System OFF".
		   In short, don't expect schedule() to work.
		*/
		loop { cortex_m::asm::wfi(); }
		// loop {}
	}

	#[task(resources = [rtc, ui, usbctl_proxy, fido_app])]
	fn frontend(ctx: frontend::Context) {
		let usbctl = ctx.resources.usbctl_proxy.access();

		if let ui::USBControllerEnum::Real(r) = usbctl {
			if r.poll_apps(&mut [ctx.resources.fido_app]) {
				rtic::pend(nrf52840_hal::pac::Interrupt::USBD);
			}
		}

		/*
		   This is the function where we perform least-urgency stuff, like rendering
		   display contents.
		*/
		let rtc_time = ctx.resources.rtc.get_counter();
		ctx.resources.ui.refresh(rtc_time);
	}

	#[task(binds = SWI0_EGU0, resources = [trussed_service])]
	fn irq_trussed(ctx: irq_trussed::Context) {
		ctx.resources.trussed_service.process();
	}

	#[task(binds = GPIOTE, resources = [uart, power, gpiote, ui])]
	fn irq_gpiote(ctx: irq_gpiote::Context) {
		rtt_target::rprintln!("RTIC GPIO IRQ");
		ctx.resources.ui.check_buttons();
		ctx.resources.gpiote.reset_events();
	}

	#[task(binds = USBD, resources = [usbctl_proxy])]
	fn usb_handler(ctx: usb_handler::Context) {
		rtt_target::rprintln!("USB");

		let usbctl = ctx.resources.usbctl_proxy.access();

		if let ui::USBControllerEnum::Real(r) = usbctl {
			r.poll();
		}
	}

	#[task(binds = RTC0, resources = [uart, rtc], spawn = [frontend])]
	fn rtc_handler(ctx: rtc_handler::Context) {
		let rtc_count = ctx.resources.rtc.get_counter();
		rtt_target::rprintln!("RTC");
		uart_emit(ctx.resources.uart, b"RTCctr ", Some(rtc_count));
		ctx.resources.rtc.reset_event(RtcInterrupt::Tick);
		ctx.spawn.frontend().ok();
	}

	#[task(binds = POWER_CLOCK, resources = [power, uart, usbctl_proxy])]
	fn power_handler(ctx: power_handler::Context) {
		let mut pwr: u32;
		rtt_target::rprintln!("POWER");
		pwr = ctx.resources.power.mainregstatus.read().bits();
		uart_emit(ctx.resources.uart, b"PWRm ", Some(pwr));
		pwr = ctx.resources.power.usbregstatus.read().bits();
		uart_emit(ctx.resources.uart, b"PWRu ", Some(pwr));
		pwr = ctx.resources.power.pofcon.read().bits();
		uart_emit(ctx.resources.uart, b"POF  ", Some(pwr));

		if ctx.resources.power.events_usbdetected.read().events_usbdetected().bits() {
			ctx.resources.usbctl_proxy.instantiate();
			ctx.resources.power.events_usbdetected.write(|w| unsafe { w.bits(0) });
		}

		if ctx.resources.power.events_usbpwrrdy.read().events_usbpwrrdy().bits() {
			ctx.resources.power.events_usbpwrrdy.write(|w| unsafe { w.bits(0) });
		}

		if ctx.resources.power.events_usbremoved.read().events_usbremoved().bits() {
			ctx.resources.usbctl_proxy.deinstantiate();
			ctx.resources.power.events_usbremoved.write(|w| unsafe { w.bits(0) });
		}
	}

	extern "C" {
		fn SWI5_EGU5();
	}
};

const CRLF: [u8; 2] = [13, 10];

pub fn u32_to_hex08(v: u32, buf: &mut [u8; 8], pad: bool) -> &[u8] {
	const HEX: [u8; 16] = [48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 97, 98, 99, 100, 101, 102];
	let mut i: usize = 8;
	let mut vv: u32 = v;

	while i > 0 {
		i -= 1;
		buf[i] = HEX[(vv & 15) as usize];
		vv >>= 4;
		if (!pad) && (vv == 0) {
			return &buf[i+1..];
		}
	}

	return &buf[..];
}

pub fn uart_emit(uart: &mut Uarte<nrf52840_hal::pac::UARTE0>, buf: &[u8], val: Option<u32>) {
	let mut res: Result<_, _>;
	let mut mutbuf = [0u8; 8];
	let mut i: usize = 0;

	while (i < 8) && (i < buf.len()) {
		mutbuf[i] = buf[i];
		i += 1;
	}

	res = uart.write(&mutbuf[0..i]);

	if res.is_ok() {
		if let Some(iv) = val {
			res = uart.write(u32_to_hex08(iv, &mut mutbuf, false));
		}
	}

	if res.is_ok() {
		mutbuf[0] = CRLF[0];
		mutbuf[1] = CRLF[1];
		res = uart.write(&mutbuf[0..2]);
	}

	match res {
		Ok(_) => (),
		Err(nrf52840_hal::uarte::Error::BufferNotInRAM) => {
			rtt_target::rprintln!("TXE0");
		}
		Err(nrf52840_hal::uarte::Error::TxBufferTooLong) => {
			rtt_target::rprintln!("TXE1");
		}
		Err(nrf52840_hal::uarte::Error::RxBufferTooLong) => {
			rtt_target::rprintln!("TXE2");
		}
		Err(nrf52840_hal::uarte::Error::Receive) => {
			rtt_target::rprintln!("TXE3");
		}
		Err(nrf52840_hal::uarte::Error::Transmit) => {
			rtt_target::rprintln!("TXE4");
		}
		Err(nrf52840_hal::uarte::Error::Timeout(_)) => {
			rtt_target::rprintln!("TXE5");
		}
	}
}
