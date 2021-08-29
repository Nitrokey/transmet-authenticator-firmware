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
	twim::Twim,
	uarte::{Baudrate, Parity, Stopbits, Uarte},
};
use rand_core::SeedableRng;
use rtic::cyccnt::Instant;
use trussed::{
	Interchange,
	types::{LfsResult, LfsStorage},
};

#[cfg(not(any(feature = "board-nrfdk", feature = "board-proto1")))]
compile_error!{"No board target chosen! Set your board using --feature; see Cargo.toml."}

#[cfg_attr(feature = "board-nrfdk", path = "board_nrfdk.rs")]
#[cfg_attr(feature = "board-proto1", path = "board_proto1.rs")]
mod board;

mod flash;
mod se050;
mod types;
mod ui;
mod usb;

/* TODO: add external flash */
littlefs2::const_ram_storage!(ExternalStore, 1024);
littlefs2::const_ram_storage!(VolatileStore, 8192);
trussed::store!(
	StickStore,
	Internal: flash::FlashStorage,
	External: ExternalStore,
	Volatile: VolatileStore
);

trussed::platform!(
	StickPlatform,
	R: chacha20::ChaCha8Rng,
	S: StickStore,
	UI: ui::WrappedUI,
);

pub struct NRFSyscall {}
impl trussed::platform::Syscall for NRFSyscall {
	fn syscall(&mut self) {
		rtt_target::rprintln!("SYS");
		rtic::pend(nrf52840_hal::pac::Interrupt::SWI0_EGU0);
	}
}

pub struct NRFReboot {}
impl admin_app::Reboot for NRFReboot {
	fn reboot() -> ! { todo!() }
	fn reboot_to_firmware_update() -> ! { todo!() }
	fn reboot_to_firmware_update_destructive() -> ! { todo!() }
}

type TrussedClient = trussed::ClientImplementation<NRFSyscall>;

enum FrontendOp {
	RefreshUI(u32),
	SetBatteryState(ui::StickBatteryState),
}

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
	struct Resources {
		gpiote: Gpiote,
		ui: ui::StickUI,
		trussed_service: trussed::service::Service<StickPlatform>,
		uart: Uarte<nrf52840_hal::pac::UARTE0>,
		pre_usb: Option<usb::USBPreinitObjects>,
		#[init(None)]
		usb: Option<usb::USBObjects<'static>>,
		#[init(None)]
		usb_dispatcher: Option<usb::USBDispatcher>,
		power: nrf52840_hal::pac::POWER,
		rtc: Rtc<nrf52840_hal::pac::RTC0>,
		fido_app: dispatch_fido::Fido<fido_authenticator::NonSilentAuthenticator, TrussedClient>,
		admin_app: admin_app::App<TrussedClient, NRFReboot>,
	}

	#[init(spawn = [frontend])]
	fn init(mut ctx: init::Context) -> init::LateResources {
		ctx.core.DCB.enable_trace();
		ctx.core.DWT.enable_cycle_counter();

		rtt_target::rtt_init_print!();

		let ficr = &*ctx.device.FICR;
		rtt_target::rprintln!("FICR DeviceID {:08x} {:08x}", ficr.deviceid[0].read().bits(), ficr.deviceid[1].read().bits());
		rtt_target::rprintln!("FICR EncRoot  {:08x} {:08x} {:08x} {:08x}",
			ficr.er[0].read().bits(), ficr.er[1].read().bits(),
			ficr.er[2].read().bits(), ficr.er[3].read().bits());
		rtt_target::rprintln!("FICR IdtRoot  {:08x} {:08x} {:08x} {:08x}",
			ficr.ir[0].read().bits(), ficr.ir[1].read().bits(),
			ficr.ir[2].read().bits(), ficr.ir[3].read().bits());
		let da0 = ficr.deviceaddr[0].read().bits();
		let da1 = ficr.deviceaddr[1].read().bits();
		rtt_target::rprintln!("FICR DevAddr  {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} {}",
			(da1 >> 8) as u8, da1 as u8,
			(da0 >> 24) as u8, (da0 >> 16) as u8, (da0 >> 8) as u8, da0 as u8,
			if (ficr.deviceaddrtype.read().bits() & 1) != 0 { "RND" } else { "PUB" });

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
		gpiote.reset_events();

		rtt_target::rprintln!("UART");

		let uart = Uarte::new(ctx.device.UARTE0, board_gpio.uart_pins.take().unwrap(),
				Parity::EXCLUDED, Baudrate::BAUD57600, Stopbits::TWO
		);

		rtt_target::rprintln!("Display");

		if board_gpio.display_power.is_some() {
			use nrf52840_hal::prelude::OutputPin;
			board_gpio.display_power.as_mut().unwrap().set_low().ok();
		}
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

		rtt_target::rprintln!("Secure Element");

		if board_gpio.se_pins.is_some() {
			let twim1 = Twim::new(ctx.device.TWIM1, board_gpio.se_pins.take().unwrap(), nrf52840_hal::twim::Frequency::K400);
			let mut secelem = se050::Se050::new(twim1, board_gpio.se_power.take().unwrap());
			secelem.enable();
		}

		rtt_target::rprintln!("Flash");

		let mut stickflash = flash::FlashStorage::new(ctx.device.NVMC, 0x000E_0000 as *mut u32, flash::FLASH_SIZE as usize);
		if cfg!(feature = "reformat-flash") {
			rtt_target::rprintln!("--> ERASING FLASH");
			stickflash.erase(0, flash::FLASH_SIZE).ok();
		}

		rtt_target::rprintln!("Trussed Store");

		let stickstore = StickStore::init(
			stickflash,
			ExternalStore::new(),
			VolatileStore::new(),
			cfg!(feature = "reformat-flash")
		);

		// let foopath = littlefs2::path::PathBuf::from("testme/dat/rng-state.bin");
		// trussed::store::store(stickstore, trussed::types::Location::Internal, &foopath, &[0u8; 32]).ok();

		rtt_target::rprintln!("Trussed Platform");

		let stickplat = StickPlatform::new(
			chacha20::ChaCha8Rng::from_rng(rng).unwrap(),
			stickstore,
			ui::WrappedUI::new()
		);

		rtt_target::rprintln!("Trussed Service");

		let mut srv = trussed::service::Service::new(stickplat);

		rtt_target::rprintln!("Apps");

		let fido_trussed_xch = trussed::pipe::TrussedInterchange::claim().unwrap();
		let fido_lfs2_path = littlefs2::path::PathBuf::from("fido");
		srv.add_endpoint(fido_trussed_xch.1, fido_lfs2_path).ok();
		let fido_trussed_client = TrussedClient::new(fido_trussed_xch.0, NRFSyscall {});
		let fido_auth = fido_authenticator::Authenticator::new(fido_trussed_client, fido_authenticator::NonSilentAuthenticator {});
		let fido_app = dispatch_fido::Fido::<fido_authenticator::NonSilentAuthenticator, TrussedClient>::new(fido_auth);

		let admin_trussed_xch = trussed::pipe::TrussedInterchange::claim().unwrap();
		let admin_lfs2_path = littlefs2::path::PathBuf::from("admin");
		srv.add_endpoint(admin_trussed_xch.1, admin_lfs2_path).ok();
		let admin_trussed_client = TrussedClient::new(admin_trussed_xch.0, NRFSyscall {});
		let admin_app = admin_app::App::<TrussedClient, NRFReboot>::new(admin_trussed_client, [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15], 0x00000001);

		rtt_target::rprintln!("USB");

		let clocks = Clocks::new(ctx.device.CLOCK).start_lfclk().enable_ext_hfosc();

		let usb_preinit = usb::preinit(ctx.device.USBD, clocks);

		rtt_target::rprintln!("Fingerprint Reader");

		if board_gpio.fpr_power.is_some() {
			use nrf52840_hal::prelude::OutputPin;
			board_gpio.fpr_power.as_mut().unwrap().set_low().ok();
		}

		rtt_target::rprintln!("Finalizing");

		// RTIC enables the interrupt during init if there is a handler function bound to it
		rtc.enable_interrupt(RtcInterrupt::Tick, None);
		rtc.enable_counter();

		gpiote.port().enable_interrupt();
		power.intenset.write(|w| w.pofwarn().set_bit().usbdetected().set_bit().usbremoved().set_bit().usbpwrrdy().set_bit());

		init::LateResources {
			gpiote,
			ui,
			trussed_service: srv,
			uart,
			pre_usb: Some(usb_preinit),
			power,
			rtc,
			fido_app,
			admin_app
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
		rtt_target::rprintln!("idle");
		loop { cortex_m::asm::wfi(); }
		// loop {}
	}

	#[task(priority = 1, resources = [ui], capacity = 4)]
	fn frontend(ctx: frontend::Context, op: FrontendOp) {
		let frontend::Resources { ui } = ctx.resources;

		/*
		   This is the function where we perform least-urgency stuff, like rendering
		   display contents.
		*/
		match op {
		FrontendOp::RefreshUI(x) => { ui.refresh(x); },
		FrontendOp::SetBatteryState(x) => { ui.set_battery(x); }
		}
	}

	#[task(priority = 1, resources = [usb_dispatcher, fido_app, admin_app])]
	fn userspace_apps(ctx: userspace_apps::Context) {
		let userspace_apps::Resources { usb_dispatcher, fido_app, admin_app} = ctx.resources;

		rtt_target::rprintln!("UA");
		//usb_dispatcher.lock(|usb_dispatcher| {
		if usb_dispatcher.is_some() {
			// cortex_m::peripheral::NVIC::mask(nrf52840_hal::pac::Interrupt::USBD);
			let b = usb_dispatcher.as_mut().unwrap().poll_apps(&mut [fido_app, admin_app]);
			if b {
				rtt_target::rprintln!("rUSB");
				rtic::pend(nrf52840_hal::pac::Interrupt::USBD);
			}
			// unsafe { cortex_m::peripheral::NVIC::unmask(nrf52840_hal::pac::Interrupt::USBD); }
		}
		//});
	}

	#[task(priority = 1, resources = [pre_usb, usb, usb_dispatcher])]
	fn late_setup_usb(ctx: late_setup_usb::Context) {
		let late_setup_usb::Resources { pre_usb, mut usb, usb_dispatcher } = ctx.resources;

		rtt_target::rprintln!("create USB");
		usb.lock(|usb| {
			let usb_preinit = pre_usb.take().unwrap();
			let ( usb_init, usb_dsp ) = usb::init(usb_preinit);
			usb.replace(usb_init);
			usb_dispatcher.replace(usb_dsp);
		});
	}

	#[task(priority = 2, binds = SWI0_EGU0, resources = [trussed_service])]
	fn irq_trussed(ctx: irq_trussed::Context) {
		rtt_target::rprintln!("irq SYS");
		ctx.resources.trussed_service.process();
	}

	#[task(priority = 1, binds = GPIOTE, resources = [ui, gpiote, uart])]
	fn irq_gpiote(ctx: irq_gpiote::Context) {
		rtt_target::rprintln!("irq GPIO");
		ctx.resources.ui.check_buttons();
		ctx.resources.gpiote.reset_events();

		if cfg!(feature = "board-proto1") {
			let pkt: [u8; 11+1] = [0xef, 0x01 /* magic */,
						0xff, 0xff, 0xff, 0xff /* device address */,
						0x01 /* COMMAND */,
						0x00, 0x03 /* len */,
						0x0f /* ReadSysPara */,
						0x00, 0x13 /* checksum */];
			ctx.resources.uart.write(&pkt).ok();
			let mut rpkt: [u8; 11+16] = [0u8; 27];
			ctx.resources.uart.read(&mut rpkt).ok();
			for i in 0..27 {
				rtt_target::rprintln!("UART R {:x}", rpkt[i]);
			}
		}

	}

	#[task(priority = 3, binds = USBD, resources = [usb])]
	fn usb_handler(ctx: usb_handler::Context) {
		let e0 = Instant::now();
		let ev0 = usb::usbd_debug_events();

		ctx.resources.usb.as_mut().unwrap().poll();

		let ev1 = usb::usbd_debug_events();
		let e1 = Instant::now();
		let ed = (e1 - e0).as_cycles();
		if ed > 64_000 {
			rtt_target::rprintln!("!! long top half: {:x} cyc", ed);
		}
		if (ev0 & ev1 & 0x00e0_0401) != 0 {
			rtt_target::rprintln!("USB screams, {:x} -> {:x}", ev0, ev1);
		} else {
			// rtt_target::rprintln!("irq USB {:x}", usb::usbd_debug_events());
		}
	}

	#[task(priority = 4, binds = RTC0, resources = [rtc], spawn = [frontend, userspace_apps])]
	fn rtc_handler(ctx: rtc_handler::Context) {
		let rtc_count = ctx.resources.rtc.get_counter();
		rtt_target::rprintln!("irq RTC {:x}", rtc_count);
		ctx.resources.rtc.reset_event(RtcInterrupt::Tick);
		let rtc_time = ctx.resources.rtc.get_counter();
		ctx.spawn.frontend(FrontendOp::RefreshUI(rtc_time)).ok();
		ctx.spawn.userspace_apps().ok();
	}

	#[task(priority = 3, binds = POWER_CLOCK, resources = [power], spawn = [frontend, late_setup_usb])]
	fn power_handler(ctx: power_handler::Context) {
		let power = &ctx.resources.power;
		let pwrM = power.mainregstatus.read().bits();
		let pwrU = power.usbregstatus.read().bits();
		let pof = power.pofcon.read().bits();
		rtt_target::rprintln!("irq PWR {:x} {:x} {:x}", pwrM, pwrU, pof);

		if power.events_usbdetected.read().events_usbdetected().bits() {
			ctx.spawn.late_setup_usb().ok();
			// instantiate();
			power.events_usbdetected.write(|w| unsafe { w.bits(0) });
			ctx.spawn.frontend(FrontendOp::SetBatteryState(ui::StickBatteryState::Charging(10))).ok();
		}

		if power.events_usbpwrrdy.read().events_usbpwrrdy().bits() {
			power.events_usbpwrrdy.write(|w| unsafe { w.bits(0) });
		}

		if power.events_usbremoved.read().events_usbremoved().bits() {
			// deinstantiate();
			power.events_usbremoved.write(|w| unsafe { w.bits(0) });
			ctx.spawn.frontend(FrontendOp::SetBatteryState(ui::StickBatteryState::Discharging(10))).ok();
		}
	}

	extern "C" {
		fn SWI4_EGU4();
		fn SWI5_EGU5();
	}
};
