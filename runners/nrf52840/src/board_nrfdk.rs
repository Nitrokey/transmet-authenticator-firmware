use nrf52840_pac::{
	Peripherals, CorePeripherals
};
use nrf52840_hal::{
	gpio::{p0, p1, Level},
	gpiote::Gpiote,
};

use crate::types::*;

pub fn init_early(_device: &Peripherals, _core: &CorePeripherals) -> () {

}

pub fn init_gpio(gpiote: &Gpiote, gpio_p0: p0::Parts, gpio_p1: p1::Parts) -> BoardGPIO {
	/* Button 1-4: on DK */
	let btn1 = gpio_p0.p0_11.into_pullup_input().degrade();
	let btn2 = gpio_p0.p0_12.into_pullup_input().degrade();
	let btn3 = gpio_p0.p0_24.into_pullup_input().degrade();
	let btn4 = gpio_p0.p0_25.into_pullup_input().degrade();

	/* Button 5-8: wired through from Pico LCD */
	let btn5 = gpio_p1.p1_08.into_pullup_input().degrade();
	let btn6 = gpio_p1.p1_07.into_pullup_input().degrade();
	let btn7 = gpio_p1.p1_06.into_pullup_input().degrade();
	let btn8 = gpio_p1.p1_05.into_pullup_input().degrade();

	gpiote.port().input_pin(&btn1).low();
	gpiote.port().input_pin(&btn2).low();
	gpiote.port().input_pin(&btn3).low();
	gpiote.port().input_pin(&btn4).low();
	gpiote.port().input_pin(&btn5).low();
	gpiote.port().input_pin(&btn6).low();
	gpiote.port().input_pin(&btn7).low();
	gpiote.port().input_pin(&btn8).low();

	/* LEDs */
	let led1 = gpio_p0.p0_13.into_push_pull_output(Level::High).degrade();
	let led2 = gpio_p0.p0_14.into_push_pull_output(Level::High).degrade();
	let led3 = gpio_p0.p0_15.into_push_pull_output(Level::High).degrade();
	let led4 = gpio_p0.p0_16.into_push_pull_output(Level::High).degrade();

	/* UART */
	let u_rx = gpio_p0.p0_08.into_floating_input().degrade();
	let u_tx = gpio_p0.p0_06.into_push_pull_output(Level::High).degrade();

	/* Display SPI Bus */
	let dsp_spi_dc = gpio_p1.p1_10.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_cs = gpio_p1.p1_11.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_clk = gpio_p1.p1_12.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_mosi = gpio_p1.p1_13.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_rst = gpio_p1.p1_14.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_bl = gpio_p1.p1_15.into_push_pull_output(Level::High).degrade();
	// no power gate

	BoardGPIO { buttons: [
			Some(btn1), Some(btn2), Some(btn3), Some(btn4),
			Some(btn5), Some(btn6), Some(btn7), Some(btn8) ],
		leds: [ Some(led1), Some(led2), Some(led3), Some(led4) ],
		uart_rx: Some(u_rx),
		uart_tx: Some(u_tx),
		uart_cts: None,
		uart_rts: None,
		display_spi: [
			Some(dsp_spi_bl), Some(dsp_spi_clk), Some(dsp_spi_cs), Some(dsp_spi_dc),
			Some(dsp_spi_mosi), None, Some(dsp_spi_rst) ],
		display_spi_miso: None,
	}
}
