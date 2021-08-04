use nrf52840_hal::{
	gpio::{Pin, Input, Floating, Output, PushPull, PullUp},
	spim,
};

pub struct BoardGPIO {
	pub buttons: [Option<Pin<Input<PullUp>>>; 8],
	pub leds: [Option<Pin<Output<PushPull>>>; 4],
	pub uart_rx: Option<Pin<Input<Floating>>>,
	pub uart_tx: Option<Pin<Output<PushPull>>>,
	pub uart_cts: Option<Pin<Input<Floating>>>,
	pub uart_rts: Option<Pin<Output<PushPull>>>,
	pub display_spi: Option<spim::Pins>,
	pub display_cs: Option<Pin<Output<PushPull>>>,
	pub display_reset: Option<Pin<Output<PushPull>>>,
	pub display_dc: Option<Pin<Output<PushPull>>>,
	pub display_backlight: Option<Pin<Output<PushPull>>>,
	pub display_power: Option<Pin<Output<PushPull>>>,
	pub flashnfc_spi: Option<spim::Pins>,
	pub flash_cs: Option<Pin<Output<PushPull>>>,
	pub flash_power: Option<Pin<Output<PushPull>>>,
	pub nfc_cs: Option<Pin<Output<PushPull>>>,
	pub nfc_irq: Option<Pin<Input<PullUp>>>,
}
