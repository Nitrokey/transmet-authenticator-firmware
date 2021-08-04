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
	/* Buttons */
	let btn1 = gpio_p1.p1_11.into_pullup_input().degrade();
	let btn2 = gpio_p1.p1_13.into_pullup_input().degrade();
	let btn3 = gpio_p1.p1_15.into_pullup_input().degrade();
	/* btn4 = p1_10 -- do not use, to be removed later */

	gpiote.port().input_pin(&btn1).low();
	gpiote.port().input_pin(&btn2).low();
	gpiote.port().input_pin(&btn3).low();

	/* Display SPI Bus */
	let dsp_spi_dc = gpio_p0.p0_26.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_cs = gpio_p0.p0_06.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_clk = gpio_p0.p0_01.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_mosi = gpio_p0.p0_00.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_rst = gpio_p0.p0_04.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_bl = gpio_p0.p0_08.into_push_pull_output(Level::High).degrade();
	let dsp_spi_pwr = gpio_p0.p0_13.into_push_pull_output(Level::High).degrade();

	BoardGPIO { buttons: [
			Some(btn1), Some(btn2), Some(btn3), None,
			None, None, None, None ],
		leds: [ None, None, None, None ],
		uart_rx: None,
		uart_tx: None,
		uart_cts: None,
		uart_rts: None,
		display_spi: [
			Some(dsp_spi_bl), Some(dsp_spi_clk), Some(dsp_spi_cs), Some(dsp_spi_dc),
			Some(dsp_spi_mosi), Some(dsp_spi_pwr), Some(dsp_spi_rst) ],
		display_spi_miso: None,
	}
}
