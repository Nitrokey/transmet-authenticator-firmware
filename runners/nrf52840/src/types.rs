use nrf52840_hal::{
	gpio::{Pin, Input, Floating, Output, PushPull, PullUp},
};

pub struct BoardGPIO {
	pub buttons: [Option<Pin<Input<PullUp>>>; 8],
	pub leds: [Option<Pin<Output<PushPull>>>; 4],
	pub uart_rx: Option<Pin<Input<Floating>>>,
	pub uart_tx: Option<Pin<Output<PushPull>>>,
	pub uart_cts: Option<Pin<Input<Floating>>>,
	pub uart_rts: Option<Pin<Output<PushPull>>>,
	pub display_spi: [Option<Pin<Output<PushPull>>>; 7],
	pub display_spi_miso: Option<Pin<Input<Floating>>>,
}

pub mod display_spi_pins {
	pub const DISPLAY_SPI_BL: usize = 0;
	pub const DISPLAY_SPI_CLK: usize = 1;
	#[allow(dead_code)]
	pub const DISPLAY_SPI_CS: usize = 2;
	pub const DISPLAY_SPI_DC: usize = 3;
	pub const DISPLAY_SPI_MOSI: usize = 4;
	pub const DISPLAY_SPI_POWER: usize = 5;
	pub const DISPLAY_SPI_RST: usize = 6;
}
