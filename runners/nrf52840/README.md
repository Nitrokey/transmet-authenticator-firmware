# NRF52840 port of the Solo2 firmware

## LPC55 bootup

src/main.rs:		RTIC App init()
-> src/lib.rs:		init_board()
   -> src/initializer.rs:	Initializer::Config {}
   -> src/initializer.rs:	Initializer::new()
   -> src/initializer.rs:	Initializer::initialize_all()
      -> src/initializer.rs:	initialize_clocks()
         ->			enable_clocks()
      -> src/initializer.rs:	initialize_basic()
      -> src/initializer.rs:	initialize_nfc()
      -> src/initializer.rs:	initialize_usb()
         -> ???			apdu_dispatch::interchanges::Contact::claim()
         -> ???			ctaphid_dispatch::types::HidInterchange::claim()
         -> ???			hal::drivers::UsbBus::new()
         -> ???			usbd_ccid::Ccid::new()
         -> ???			usbd_ctaphid::CtapHid::new()
         -> ???			usbd_serial::SerialPort::new()
         -> ???			UsbDeviceBuilder::new()
      -> src/initializer.rs:	initialize_interfaces()
         -> ???			types::ApduDispatch::new()
         -> ???			types::CtaphidDispatch::new()
      -> src/initializer.rs:	initialize_flash()
      -> src/initializer.rs:	initialize_filesystem()
      -> src/initializer.rs:	initialize_trussed()
   -> src/initializer.rs:	Initializer::get_dynamic_clock_control()
   -> ???			types::Apps::new()

==============================================================================

src/main.rs:		RTIC App idle()
-> ???			apps.apdu_dispatch(|a| apu_dispatch.poll(a))
-> ???			apps.ctaphid_dispatch(|a| ctaphid_dispatch.poll(a))
-> ???			usb_classes.poll()
