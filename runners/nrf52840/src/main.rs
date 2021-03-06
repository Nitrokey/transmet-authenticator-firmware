#![no_std]
#![no_main]

use panic_halt as _;
// use cortex_m;
use asm_delay::bitrate::U32BitrateExt;
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use littlefs2::const_ram_storage;
use nrf52840_hal::{
	clocks::Clocks,
	gpiote::Gpiote,
	prelude::OutputPin,
	rng::Rng,
	rtc::{Rtc, RtcInterrupt},
	spim::Spim,
	twim::Twim,
	uarte::{Baudrate, Parity, Uarte},
};
use rand_core::SeedableRng;
use rtic::cyccnt::Instant;
use trussed::{
	Interchange,
	types::{LfsResult, LfsStorage},
};

#[macro_use]
extern crate delog;
delog::generate_macros!();

#[cfg(not(any(feature = "board-nrfdk", feature = "board-proto1")))]
compile_error!{"No board target chosen! Set your board using --feature; see Cargo.toml."}

#[cfg_attr(feature = "board-nrfdk", path = "board_nrfdk.rs")]
#[cfg_attr(feature = "board-proto1", path = "board_proto1.rs")]
mod board;

mod extflash;
mod flash;
mod fpr;
mod se050;
mod types;
mod ui;
mod usb;

#[derive(Debug)]
pub struct NRFDelogFlusher {}
impl delog::Flusher for NRFDelogFlusher {
	fn flush(&self, s: &str) {
		rtt_target::rprint!(s);
	}
}
static NRFDELOG_FLUSHER: NRFDelogFlusher = NRFDelogFlusher {};
delog::delog!(NRFDelogger, 2*1024, 512, NRFDelogFlusher);

/* TODO: add external flash */
littlefs2::const_ram_storage!(ExternalRAMStore, 1024);
littlefs2::const_ram_storage!(VolatileRAMStore, 8192);
trussed::store!(
	StickStore,
	Internal: flash::FlashStorage,
	External: ExternalRAMStore,
	Volatile: VolatileRAMStore
);

unsafe impl Send for StickStore {}

trussed::platform!(
	StickPlatform,
	R: chacha20::ChaCha8Rng,
	S: StickStore,
	UI: ui::WrappedUI,
);

pub struct NRFSyscall {}
impl trussed::platform::Syscall for NRFSyscall {
	fn syscall(&mut self) {
		// trace!("SYS");
		rtic::pend(nrf52840_hal::pac::Interrupt::SWI0_EGU0);
	}
}

pub struct NRFReboot {}
impl admin_app::Reboot for NRFReboot {
	fn reboot() -> ! { todo!() }
	fn reboot_to_firmware_update() -> ! { todo!() }
	fn reboot_to_firmware_update_destructive() -> ! { todo!() }
}

type TrussedNRFClient = trussed::ClientImplementation<NRFSyscall>;

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
		finger: Option<fpr::FingerprintReader<nrf52840_hal::pac::UARTE0>>,
		pre_usb: Option<usb::USBPreinitObjects>,
		#[init(None)]
		usb: Option<usb::USBObjects<'static>>,
		#[init(None)]
		usb_dispatcher: Option<usb::USBDispatcher>,
		extflash: Option<extflash::ExtFlashStorage<nrf52840_hal::spim::Spim<nrf52840_hal::pac::SPIM3>>>,
		se050: Option<se050::Se050<nrf52840_hal::pac::TWIM1>>,
		power: nrf52840_hal::pac::POWER,
		rtc: Rtc<nrf52840_hal::pac::RTC0>,
		fido_app: dispatch_fido::Fido<fido_authenticator::NonSilentAuthenticator, TrussedNRFClient>,
		admin_app: admin_app::App<TrussedNRFClient, NRFReboot>,
		piv_app: piv_authenticator::Authenticator<TrussedNRFClient, {apdu_dispatch::command::SIZE}>,
		prov_app: provisioner_app::Provisioner<StickStore, flash::FlashStorage, TrussedNRFClient>,
	}

	#[init(spawn = [frontend])]
	fn init(mut ctx: init::Context) -> init::LateResources {
		ctx.core.DCB.enable_trace();
		ctx.core.DWT.enable_cycle_counter();

		rtt_target::rtt_init_print!();
		NRFDelogger::init_default(delog::LevelFilter::Trace, &NRFDELOG_FLUSHER).ok();

		let ficr = &*ctx.device.FICR;
		let mut device_uuid: [u8; 16] = [0u8; 16];
		device_uuid[0..4].copy_from_slice(&ficr.deviceid[0].read().bits().to_be_bytes());
		device_uuid[4..8].copy_from_slice(&ficr.deviceid[1].read().bits().to_be_bytes());
		info!("FICR DeviceID {:08x} {:08x}", ficr.deviceid[0].read().bits(), ficr.deviceid[1].read().bits());
		info!("FICR EncRoot  {:08x} {:08x} {:08x} {:08x}",
			ficr.er[0].read().bits(), ficr.er[1].read().bits(),
			ficr.er[2].read().bits(), ficr.er[3].read().bits());
		info!("FICR IdtRoot  {:08x} {:08x} {:08x} {:08x}",
			ficr.ir[0].read().bits(), ficr.ir[1].read().bits(),
			ficr.ir[2].read().bits(), ficr.ir[3].read().bits());
		let da0 = ficr.deviceaddr[0].read().bits();
		let da1 = ficr.deviceaddr[1].read().bits();
		info!("FICR DevAddr  {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} {}",
			(da1 >> 8) as u8, da1 as u8,
			(da0 >> 24) as u8, (da0 >> 16) as u8, (da0 >> 8) as u8, da0 as u8,
			if (ficr.deviceaddrtype.read().bits() & 1) != 0 { "RND" } else { "PUB" });
		info!("RESET Reason: {:08x}", ctx.device.POWER.resetreas.read().bits());
		ctx.device.POWER.resetreas.write(|w| w);

		board::init_early(&ctx.device, &ctx.core);

		debug!("Peripheral Wrappers");

		let gpiote = Gpiote::new(ctx.device.GPIOTE);
		let p0 = nrf52840_hal::gpio::p0::Parts::new(ctx.device.P0);
		let p1 = nrf52840_hal::gpio::p1::Parts::new(ctx.device.P1);
		let rng = Rng::new(ctx.device.RNG);
		let power = ctx.device.POWER;
		let mut rtc = Rtc::new(ctx.device.RTC0, 4095).unwrap();

		debug!("Pins");

		let mut board_gpio = board::init_gpio(&gpiote, p0, p1);
		gpiote.reset_events();

		debug!("UART");

		let uart = Uarte::new(ctx.device.UARTE0, board_gpio.uart_pins.take().unwrap(),
				Parity::EXCLUDED, Baudrate::BAUD57600
		);

		debug!("Display");

		if board_gpio.display_power.is_some() {
			board_gpio.display_power.as_mut().unwrap().set_low().ok();
		}
		let spi = Spim::new(ctx.device.SPIM0, board_gpio.display_spi.take().unwrap(),
			nrf52840_hal::spim::Frequency::M8,
			nrf52840_hal::spim::MODE_3,
			0x7e_u8,
		);
		let di_spi = display_interface_spi::SPIInterfaceNoCS::new(spi, board_gpio.display_dc.take().unwrap());
		let dsp_st7789 = picolcd114::ST7789::new(di_spi, board_gpio.display_reset.take().unwrap(), 240, 135, 40, 53);

		// dsp_st7789.init(&mut delay_provider).unwrap();

		let disp = ui::Display::new(dsp_st7789,
				board_gpio.display_backlight.take().unwrap(),
				board_gpio.display_power.take());
		let ui = ui::StickUI::new(disp, board_gpio.buttons, board_gpio.leds);

		/* WIP: put together our hacked up LEGO bricks to create the Trussed service instance */

		debug!("Secure Element");

		let se050 = if board_gpio.se_pins.is_some() {
			let twim1 = Twim::new(ctx.device.TWIM1, board_gpio.se_pins.take().unwrap(), nrf52840_hal::twim::Frequency::K400);
			let mut secelem = se050::Se050::new(twim1, board_gpio.se_power.take().unwrap());
			secelem.enable().expect("SE050 ERROR");
			Some(secelem)
		} else { None };

		debug!("Internal Flash");

		let stickflash = flash::FlashStorage::new(ctx.device.NVMC, 0x000E_0000 as *mut u32, flash::FLASH_SIZE as usize);

		debug!("External Flash");

		let mut spim3 = Spim::new(ctx.device.SPIM3, board_gpio.flashnfc_spi.take().unwrap(),
			nrf52840_hal::spim::Frequency::M2,
			nrf52840_hal::spim::MODE_0,
			0x00u8,
		);
		let mut stickextflash = extflash::ExtFlashStorage::new(&mut spim3,
					board_gpio.flash_cs.take().unwrap(),
					board_gpio.flash_power);
		stickextflash.init(&mut spim3);

		debug!("Trussed Store");

		let stickstore = setup_store(stickflash, cfg!(feature = "reformat-flash"));
		let stickstore_prov = stickstore.clone();

		debug!("Trussed Platform");

		let stickplat = StickPlatform::new(
			chacha20::ChaCha8Rng::from_rng(rng).unwrap(),
			stickstore,
			ui::WrappedUI::new()
		);

		debug!("Trussed Service");

		let mut srv = trussed::service::Service::new(stickplat);

		debug!("Apps");

		let (fido_app, admin_app, piv_app, prov_app) = instantiate_apps(&mut srv, stickstore_prov, device_uuid);

		debug!("USB");

		let clocks = Clocks::new(ctx.device.CLOCK).start_lfclk().enable_ext_hfosc();

		let usb_preinit = usb::preinit(ctx.device.USBD, clocks);

		let fprx = {
		if board_gpio.fpr_power.is_some() {
			debug!("Fingerprint Reader");
			let fprx_ = fpr::FingerprintReader::new(uart, 0xffff_ffffu32,
						board_gpio.fpr_power.take().unwrap(),
						board_gpio.fpr_detect.take().unwrap());
			Some(fprx_)
		} else {
			None
		}};

		debug!("Finalizing");

		// RTIC enables the interrupt during init if there is a handler function bound to it
		rtc.enable_interrupt(RtcInterrupt::Tick, None);
		rtc.enable_counter();

		gpiote.port().enable_interrupt();
		power.intenset.write(|w| w.pofwarn().set_bit().usbdetected().set_bit().usbremoved().set_bit().usbpwrrdy().set_bit());

		init::LateResources {
			gpiote,
			ui,
			trussed_service: srv,
			finger: fprx,
			pre_usb: Some(usb_preinit),
			extflash: Some(stickextflash),
			se050,
			power,
			rtc,
			fido_app,
			admin_app,
			piv_app,
			prov_app
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
		loop {
			trace!("idle");
			cortex_m::asm::wfi();
			NRFDelogger::flush();
		}
		// loop {}
	}

	#[task(priority = 1, resources = [ui], capacity = 4)]
	#[inline(never)]
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

	#[task(priority = 1, resources = [usb_dispatcher, fido_app, admin_app, piv_app, prov_app])]
	#[inline(never)]
	fn userspace_apps(ctx: userspace_apps::Context) {
		let userspace_apps::Resources { usb_dispatcher, fido_app, admin_app, piv_app, prov_app} = ctx.resources;

		//usb_dispatcher.lock(|usb_dispatcher| {
		if usb_dispatcher.is_some() {
			cortex_m::peripheral::NVIC::mask(nrf52840_hal::pac::Interrupt::USBD);
			let (r0_usb, _r0_nfc) = usb_dispatcher.as_mut().unwrap().poll_ctaphid_apps(&mut [fido_app, admin_app]);
			let (r1_usb, _r1_nfc) = usb_dispatcher.as_mut().unwrap().poll_apdu_apps(&mut [fido_app, admin_app, piv_app, prov_app]);
			if r0_usb || r1_usb {
				trace!("rUSB");
				rtic::pend(nrf52840_hal::pac::Interrupt::USBD);
			}
			unsafe { cortex_m::peripheral::NVIC::unmask(nrf52840_hal::pac::Interrupt::USBD); }
		}
		//});
	}

	#[task(priority = 1, resources = [pre_usb, usb, usb_dispatcher])]
	#[inline(never)]
	fn late_setup_usb(ctx: late_setup_usb::Context) {
		let late_setup_usb::Resources { pre_usb, mut usb, usb_dispatcher } = ctx.resources;

		trace!("create USB");
		usb.lock(|usb| {
			let usb_preinit = pre_usb.take().unwrap();
			let ( usb_init, usb_dsp ) = usb::init(usb_preinit);
			usb.replace(usb_init);
			usb_dispatcher.replace(usb_dsp);
		});
	}

	#[task(priority = 1, resources = [usb])]
	#[inline(never)]
	fn comm_keepalives(ctx: comm_keepalives::Context) {
		let comm_keepalives::Resources { mut usb } = ctx.resources;

		usb.lock(|usb| {
			if usb.is_some() { usb.as_mut().unwrap().send_keepalives(); }
		});
	}

	#[task(priority = 2, binds = SWI0_EGU0, resources = [trussed_service])]
	fn irq_trussed(ctx: irq_trussed::Context) {
		trace!("irq SYS");
		ctx.resources.trussed_service.process();
	}

	#[task(priority = 1, binds = GPIOTE, resources = [ui, gpiote, finger, se050])]
	fn irq_gpiote(ctx: irq_gpiote::Context) {
		let irq_gpiote::Resources { ui, gpiote, finger, se050 } = ctx.resources;
		let sources: u32;
		let val_p0: u32;
		let val_p1: u32;
		unsafe {
			let pacp = nrf52840_hal::pac::Peripherals::steal();
			val_p0 = pacp.P0.in_.read().bits();
			val_p1 = pacp.P1.in_.read().bits();
			sources = board::gpio_irq_sources(&[val_p0, val_p1]);
		}
		debug!("irq GPIO {:x} {:x} -> {:x}", val_p0, val_p1, sources);
		// let buttons = ui.check_buttons(&[latch_p0, latch_p1]);
		if let Some(finger_) = finger {
			if (sources & 0b0000_0100) != 0 {
				finger_.power_up().ok();
				finger_.erase().ok();
				finger_.power_down().ok();
			} else if (sources & 0b1_0000_0000) != 0 {
				finger_.power_up().ok();
				if finger_.is_enrolled() {
					finger_.verify().ok();
				} else {
					finger_.enrol().ok();
				}
				finger_.power_down().ok();
			}
		}
		if (sources & 0b0000_0010) != 0 && se050.is_some() {
			se050.as_mut().unwrap().get_applet_id();
		}
		gpiote.reset_events();
	}

	#[task(priority = 3, binds = USBD, resources = [usb])]
	fn usb_handler(ctx: usb_handler::Context) {
		let usb_handler::Resources { usb } = ctx.resources;
		// trace!("irq USB");
		trace_now!("irq USB {:x}", usb::usbd_debug_events());

		if let Some(usb_) = usb {
			let e0 = Instant::now();
			// let ev0 = usb::usbd_debug_events();

			usb_.poll();

			// let ev1 = usb::usbd_debug_events();
			let e1 = Instant::now();

			let ed = (e1 - e0).as_cycles();
			if ed > 64_000 {
				warn!("!! long top half: {:x} cyc", ed);
			}

			/* Watched bits:
				[0]	usbreset
				[10]	ep0datadone
				[21]	sof
				[22]	usbevent
				[23]	ep0setup
				[24] --	epdata
			//
			if (ev0 & ev1 & 0x00e0_0401) != 0 {
				warn!("USB screams, {:x} -> {:x}", ev0, ev1);
			} */

			usb_.send_keepalives();
		}
	}

	#[task(priority = 4, binds = RTC0, resources = [rtc], spawn = [frontend, userspace_apps, comm_keepalives, try_system_off])]
	fn rtc_handler(ctx: rtc_handler::Context) {
		let rtc_count = ctx.resources.rtc.get_counter();
		debug!("irq RTC {:x}", rtc_count);
		ctx.resources.rtc.reset_event(RtcInterrupt::Tick);
		if (rtc_count % 2) == 0 {
			ctx.spawn.comm_keepalives().ok();
			// rtic::pend(nrf52840_hal::pac::Interrupt::SWI5_EGU5);
		}
		ctx.spawn.frontend(FrontendOp::RefreshUI(rtc_count)).ok();
		ctx.spawn.userspace_apps().ok();

		if (rtc_count >= 600*8) && (rtc_count % (10*8) == 0) {
			/* SYSTEM OFF experiments start at sysboot+60s */
			ctx.spawn.try_system_off(rtc_count).ok();
		}
	}

	#[task(priority = 3, binds = POWER_CLOCK, resources = [power], spawn = [frontend, late_setup_usb])]
	fn power_handler(ctx: power_handler::Context) {
		let power = &ctx.resources.power;
		let pwrM = power.mainregstatus.read().bits();
		let pwrU = power.usbregstatus.read().bits();
		let pof = power.pofcon.read().bits();
		debug!("irq PWR {:x} {:x} {:x}", pwrM, pwrU, pof);

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

	#[task(priority = 1, resources = [extflash, finger, ui, power, se050])]
	fn try_system_off(ctx: try_system_off::Context, c: u32) {
		let try_system_off::Resources { extflash, finger, ui, mut power, se050 } = ctx.resources;

		match c/8 {
		60 => {
			debug!("System OFF: UI");
			/* cut power to display */
			ui.power_off();
		}
		70 => {
			debug!("System OFF: FPR");
			/* cut power to fingerprint */
			finger.as_mut().unwrap().power_down().ok();
		}
		80 => {
			debug!("System OFF: EXTFLASH");
			/* cut power to external flash */
			extflash.as_mut().unwrap().power_off();
		}
		90 => {
			debug!("System OFF: SE050");
			/* cut power to SE050 */
			if let Some(se) = se050 { se.disable(); }
		}
		100 => {
			debug!("System OFF: busses+clocks");
			unsafe {
				let pac = nrf52840_hal::pac::Peripherals::steal();
				pac.SPIM0.enable.write(|w| w.bits(0));
				pac.TWIM1.enable.write(|w| w.bits(0));
				pac.SPIM3.enable.write(|w| w.bits(0));
				pac.UARTE0.enable.write(|w| w.bits(0));
				pac.USBD.enable.write(|w| w.bits(0));
				pac.CLOCK.tasks_hfclkstop.write(|w| w.bits(1));
				// pac.CLOCK.tasks_lfclkstop.write(|w| w.bits(1));
			}
		}
		110 => {
			debug!("System OFF: pins");
			unsafe {
				let pac = nrf52840_hal::pac::Peripherals::steal();
				for i in 0..64 {
					if board::is_keepalive_pin(i) {
						continue;
					}
					/* can't factor out, pac.P0 and pac.P1 have different types;
					   *sigh* Rust type safety craziness */
					if i < 32 {
						pac.P0.pin_cnf[(i & 0x1f) as usize].write(|w|
							{ w.dir().input()
							.drive().s0s1()
							.pull().disabled()
							.input().disconnect()
							.sense().disabled() });
					} else {
						pac.P1.pin_cnf[(i & 0x1f) as usize].write(|w|
							{ w.dir().input()
							.drive().s0s1()
							.pull().disabled()
							.input().disconnect()
							.sense().disabled() });
					}
				}
			}
		}
		120 => {
			debug!("System OFF");
			power.lock(|power|
				{ power.systemoff.write(|w| unsafe { w.bits(1) }); }
			);
			core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
			loop {}
		}
		_ => {}
		}
	}

	extern "C" {
		fn SWI4_EGU4();
		// fn SWI5_EGU5();
	}
};

static mut INTERNAL_STORAGE: Option<flash::FlashStorage> = None;
static mut INTERNAL_FS_ALLOC: Option<littlefs2::fs::Allocation<flash::FlashStorage>> = None;
static mut EXTERNAL_STORAGE: Option<ExternalRAMStore> = None;
static mut EXTERNAL_FS_ALLOC: Option<littlefs2::fs::Allocation<ExternalRAMStore>> = None;
static mut VOLATILE_STORAGE: Option<VolatileRAMStore> = None;
static mut VOLATILE_FS_ALLOC: Option<littlefs2::fs::Allocation<VolatileRAMStore>> = None;

fn instantiate_apps(srv: &mut trussed::service::Service<StickPlatform>, store: StickStore, device_uuid: [u8; 16]) ->
	(dispatch_fido::Fido<fido_authenticator::NonSilentAuthenticator, TrussedNRFClient>,
	admin_app::App<TrussedNRFClient, NRFReboot>,
	piv_authenticator::Authenticator<TrussedNRFClient, {apdu_dispatch::command::SIZE}>,
	provisioner_app::Provisioner<StickStore, flash::FlashStorage, TrussedNRFClient>) {
	let fido_trussed_xch = trussed::pipe::TrussedInterchange::claim().unwrap();
	let fido_lfs2_path = littlefs2::path::PathBuf::from("fido");
	srv.add_endpoint(fido_trussed_xch.1, fido_lfs2_path).ok();
	let fido_trussed_client = TrussedNRFClient::new(fido_trussed_xch.0, NRFSyscall {});
	let fido_auth = fido_authenticator::Authenticator::new(fido_trussed_client, fido_authenticator::NonSilentAuthenticator {});
	let fido_app = dispatch_fido::Fido::<fido_authenticator::NonSilentAuthenticator, TrussedNRFClient>::new(fido_auth);

	let admin_trussed_xch = trussed::pipe::TrussedInterchange::claim().unwrap();
	let admin_lfs2_path = littlefs2::path::PathBuf::from("admin");
	srv.add_endpoint(admin_trussed_xch.1, admin_lfs2_path).ok();
	let admin_trussed_client = TrussedNRFClient::new(admin_trussed_xch.0, NRFSyscall {});
	let admin_app = admin_app::App::<TrussedNRFClient, NRFReboot>::new(admin_trussed_client, device_uuid, 0x10203040);

	let piv_trussed_xch = trussed::pipe::TrussedInterchange::claim().unwrap();
	let piv_lfs2_path = littlefs2::path::PathBuf::from("piv");
	srv.add_endpoint(piv_trussed_xch.1, piv_lfs2_path).ok();
	let piv_trussed_client = TrussedNRFClient::new(piv_trussed_xch.0, NRFSyscall {});
	let piv_app = piv_authenticator::Authenticator::<TrussedNRFClient, {apdu_dispatch::command::SIZE}>::new(piv_trussed_client);

	let prov_trussed_xch = trussed::pipe::TrussedInterchange::claim().unwrap();
	let prov_lfs2_path = littlefs2::path::PathBuf::from("attn");
	srv.add_endpoint(prov_trussed_xch.1, prov_lfs2_path).ok();
	let prov_trussed_client = TrussedNRFClient::new(prov_trussed_xch.0, NRFSyscall {});
	let stolen_internal_fs = unsafe { &mut INTERNAL_STORAGE };
	let prov_app = provisioner_app::Provisioner::<StickStore, flash::FlashStorage, TrussedNRFClient>::new(prov_trussed_client, store, stolen_internal_fs.as_mut().unwrap(), false);

	(fido_app, admin_app, piv_app, prov_app)
}

fn setup_store(flash: flash::FlashStorage, reformat: bool) -> StickStore {
	unsafe {
		INTERNAL_STORAGE.replace(flash);
		INTERNAL_FS_ALLOC = Some(littlefs2::fs::Filesystem::allocate());
		EXTERNAL_STORAGE.replace(ExternalRAMStore::new());
		EXTERNAL_FS_ALLOC = Some(littlefs2::fs::Filesystem::allocate());
		VOLATILE_STORAGE.replace(VolatileRAMStore::new());
		VOLATILE_FS_ALLOC = Some(littlefs2::fs::Filesystem::allocate());
	}

	let store = StickStore::claim().unwrap();

	if reformat {
		info!("mount+format");
	} else {
		info!("mount");
	}

	store.mount(
		unsafe { INTERNAL_FS_ALLOC.as_mut().unwrap() },
		unsafe { INTERNAL_STORAGE.as_mut().unwrap() },
		unsafe { EXTERNAL_FS_ALLOC.as_mut().unwrap() },
		unsafe { EXTERNAL_STORAGE.as_mut().unwrap() },
		unsafe { VOLATILE_FS_ALLOC.as_mut().unwrap() },
		unsafe { VOLATILE_STORAGE.as_mut().unwrap() },
		reformat
	).expect("mount failed");

	/* debug!("test-store");
	let foopath = littlefs2::path::PathBuf::from("/trussed/dat/rng-state.bin");
	trussed::store::store(store, trussed::types::Location::Internal, &foopath, &[0u8; 32]).expect("foo store failed");
	*/

	store
}

/* RTIC actively hides cortex_m::peripherals::SYST from us, so we cannot use
nrf52840_hal::delay - hack around it by using a plain old
"assembly delay loop" based on the (hardcoded) CPU frequency */
pub struct Nrf52840Delay {}

impl embedded_hal::blocking::delay::DelayMs<u32> for Nrf52840Delay {
	fn delay_ms(&mut self, ms: u32) {
		let mut d = asm_delay::AsmDelay::new(64_u32.mhz());
		d.delay_ms(ms);
	}
}

impl embedded_hal::blocking::delay::DelayUs<u32> for Nrf52840Delay {
	fn delay_us(&mut self, us: u32) {
		let mut d = asm_delay::AsmDelay::new(64_u32.mhz());
		d.delay_us(us);
	}
}

pub fn board_delay(ms: u32) {
	(Nrf52840Delay {}).delay_ms(ms);
}
