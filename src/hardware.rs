//! Hardware Abstraction Module
//!
//! This module handles the low-level configuration of the RP2350 peripherals.
//! It encapsulates the complex setup of Clocks, PLLs, Timers, GPIOs, and ADC,
//! exposing a clean `Hardware` struct to the main application.

use rp235x_hal as hal;
use hal::pac;

use crate::blinky_fsm::LedPin;
use crate::usb_module;

/// External crystal frequency used by the Raspberry Pi Pico 2 W.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Initializes the entire hardware stack.
///
/// This function:
/// 1.  Takes ownership of the raw PAC peripherals.
/// 2.  Configures the Watchdog and Clocks (System & USB).
/// 3.  Initializes the Microsecond Timer.
/// 4.  Configures GPIO pins (LED).
/// 5.  Sets up the ADC for Interrupt-driven single-shot mode.
/// 6.  Initializes the USB Serial module.
///
/// # Returns
/// A tuple containing the initialized peripherals needed by `main`: `(LedPin, Timer)`.
pub fn init() -> (LedPin, hal::Timer<hal::timer::CopyableTimer0>) {
    // 1. Take ownership of raw peripherals
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // 2. Configure Clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    // 3. Configure Timer (Microsecond precision)
    let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // 4. Configure GPIOs
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let led_pin = pins.gpio15.into_push_pull_output();

    // 5. Configure ADC (Interrupt Driven)
    let _adc = hal::Adc::new(pac.ADC, &mut pac.RESETS);
    let _adc_pin = hal::adc::AdcPin::new(pins.gpio26).unwrap();

    unsafe {
        let adc_regs = &(*pac::ADC::ptr());
        
        // FIFO Control: Enable, Threshold=1, No DMA
        adc_regs.fcs().modify(|_, w| {
            w.en().set_bit()
             .thresh().bits(1)
             .dreq_en().clear_bit()
        });

        // Enable FIFO Interrupt
        adc_regs.inte().modify(|_, w| w.fifo().set_bit());

        // Channel Control: Ch0 (GPIO26), Enable, Single-Shot
        adc_regs.cs().modify(|_, w| {
            w.ainsel().bits(0)
             .en().set_bit()
             .start_many().clear_bit()
        });
        
        // Unmask ADC Interrupt in NVIC
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::ADC_IRQ_FIFO);
    }

    // 6. Configure USB Serial (via module)
    usb_module::init(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        &mut pac.RESETS,
    );

    // Return ready-to-use hardware
    (led_pin, timer)
}