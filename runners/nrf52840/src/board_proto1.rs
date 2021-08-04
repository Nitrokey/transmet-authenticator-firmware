use nrf52840_pac::{
	Peripherals, CorePeripherals
};
use nrf52840_hal::{
	gpio::{p0, p1, Level},
	gpiote::Gpiote,
	spim,
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
	let dsp_spi_cs = gpio_p0.p0_06.into_push_pull_output(Level::Low).degrade();
	let dsp_spi_clk = gpio_p0.p0_01.into_push_pull_output(Level::Low).degrade();
	/* no MISO, unidirectional SPI */
	let dsp_spi_mosi = gpio_p0.p0_00.into_push_pull_output(Level::Low).degrade();
	let dsp_rst = gpio_p0.p0_04.into_push_pull_output(Level::Low).degrade();
	let dsp_dc = gpio_p0.p0_26.into_push_pull_output(Level::Low).degrade();
	let dsp_bl = gpio_p0.p0_08.into_push_pull_output(Level::High).degrade();
	let dsp_pwr = gpio_p0.p0_13.into_push_pull_output(Level::High).degrade();

	let dsp_spi = spim::Pins {
		sck: dsp_spi_clk,
		miso: None,
		mosi: Some(dsp_spi_mosi),
	};

	/* Fingerprint */
	let _fp_tx = gpio_p0.p0_12.into_push_pull_output(Level::Low).degrade();
	let _fp_rx = gpio_p0.p0_11.into_pullup_input().degrade();
	let _fp_detect = gpio_p1.p1_09.into_pulldown_input().degrade();
	let _fp_pwr = gpio_p0.p0_15.into_push_pull_output(Level::High).degrade();

	gpiote.port().input_pin(&_fp_detect).high();

	/* Flash & NFC SPI Bus */
	let flash_spi_cs = gpio_p0.p0_25.into_push_pull_output(Level::High).degrade();
	let nfc_spi_cs = gpio_p1.p1_01.into_push_pull_output(Level::High).degrade();
	let flashnfc_spi_clk = gpio_p1.p1_02.into_push_pull_output(Level::Low).degrade();
	let flashnfc_spi_miso = gpio_p1.p1_06.into_floating_input().degrade();
	let flashnfc_spi_mosi = gpio_p1.p1_04.into_push_pull_output(Level::Low).degrade();
	let flash_pwr = gpio_p1.p1_00.into_push_pull_output(Level::Low).degrade();
	let nfc_irq = gpio_p1.p1_07.into_pullup_input().degrade();

	let flashnfc_spi = spim::Pins {
		sck: flashnfc_spi_clk,
		miso: Some(flashnfc_spi_miso),
		mosi: Some(flashnfc_spi_mosi)
	};

	BoardGPIO { buttons: [
			Some(btn1), Some(btn2), Some(btn3), None,
			None, None, None, None ],
		leds: [ None, None, None, None ],
		uart_rx: None,
		uart_tx: None,
		uart_cts: None,
		uart_rts: None,
		display_spi: Some(dsp_spi),
		display_cs: Some(dsp_spi_cs),
		display_reset: Some(dsp_rst),
		display_dc: Some(dsp_dc),
		display_backlight: Some(dsp_bl),
		display_power: Some(dsp_pwr),
		flashnfc_spi: Some(flashnfc_spi),
		flash_cs: Some(flash_spi_cs),
		flash_power: Some(flash_pwr),
		nfc_cs: Some(nfc_spi_cs),
		nfc_irq: Some(nfc_irq),
	}
}
