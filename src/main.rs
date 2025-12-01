//! SPDX-License-Identifier: MIT OR Apache-2.0
//!
//! # GPIO 'Blinky' with ADC Interrupt and USB Serial Example
//!
//! This application demonstrates a professional embedded Rust architecture using:
//! - **PAC (Peripheral Access Crate):** Direct register manipulation for advanced ADC features.
//! - **HAL (Hardware Abstraction Layer):** High-level setup for Clocks, USB, and GPIO.
//! - **Interrupts:** Non-blocking ADC reading via the `ADC_IRQ_FIFO` handler.
//! - **Critical Sections:** Safe data exchange between the interrupt context and main loop.
//! - **FSM (Finite State Machine):** Typed state machine for LED control.
//!
//! Target: Raspberry Pi Pico 2 W (RP2350).

#![no_std]
#![no_main]

// --- Imports ---
use core::cell::RefCell;
use core::fmt::Write as FmtWrite;
use critical_section::Mutex;
use defmt::*;
use defmt_rtt as _;
// use embedded_hal::digital::OutputPin; // Handled by FSM
use heapless::String;
use panic_probe as _;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

// --- Modules ---
mod blinky_fsm;
use blinky_fsm::{BlinkyContext, BlinkyEvent, BlinkyFsm};

// --- HAL Selection (Multi-target support) ---
#[cfg(rp2350)]
use rp235x_hal as hal;
#[cfg(rp2040)]
use rp2040_hal as hal;

use hal::entry;
use hal::pac;

// Select appropriate interrupt macro based on chip architecture
#[cfg(rp2350)]
use rp235x_hal::pac::interrupt;
#[cfg(rp2040)]
use rp2040_hal::pac::interrupt;

// --- Bootloader Configuration ---

/// The linker will place this boot block at the start of the program image.
/// Required for RP2040 to boot from external flash.
#[unsafe(link_section = ".boot2")]
#[used]
#[cfg(rp2040)]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

/// Program image definition for RP2350 (Secure Executable).
#[unsafe(link_section = ".start_block")]
#[used]
#[cfg(rp2350)]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External crystal frequency (Standard for Pico boards is 12MHz).
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// --- Shared State ---

/// Holds the latest ADC reading.
/// Wrapped in `Mutex<RefCell<...>>` to ensure safe access between:
/// 1. The Interrupt Handler (Writer)
/// 2. The Main Loop (Reader)
static ADC_VALUE: Mutex<RefCell<u16>> = Mutex::new(RefCell::new(0));

/// Flag indicating a new value has been captured.
static ADC_READY: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

/// Entry point. The `#[entry]` macro handles the Cortex-M startup sequence.
#[entry]
fn main() -> ! {
    info!("Program start");

    // 1. Hardware Initialization
    // ----------------------------------------------------------------
    // Take ownership of the raw peripherals (PAC level)
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Initialize the clock tree (PLLs, dividers, etc.)
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

    // Initialize the microsecond timer
    #[cfg(rp2040)]
    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    #[cfg(rp2350)]
    let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // Single-cycle IO block (required for GPIO)
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // 2. ADC Configuration (Interrupt Driven)
    // ----------------------------------------------------------------
    // Initialize the ADC peripheral
    let _adc = hal::Adc::new(pac.ADC, &mut pac.RESETS);
    // Configure GPIO26 as an Analog input (ADC Channel 0)
    let _adc_pin = hal::adc::AdcPin::new(pins.gpio26).unwrap();

    // Configure low-level ADC registers safely
    unsafe {
        let adc_regs = &(*pac::ADC::ptr());
        
        // Configure FIFO Control Status (FCS):
        // - Enable the FIFO
        // - Set threshold to 1 (interrupt fires when 1 sample is ready)
        // - Disable DMA requests
        adc_regs.fcs().modify(|_, w| {
            w.en().set_bit()
             .thresh().bits(1)
             .dreq_en().clear_bit()
        });

        // Enable the FIFO interrupt in the ADC block
        adc_regs.inte().modify(|_, w| w.fifo().set_bit());

        // Configure Control & Status (CS):
        // - Select Channel 0 (GPIO26)
        // - Enable the ADC block
        // - Ensure we are in 'one-shot' mode (not continuous)
        adc_regs.cs().modify(|_, w| {
            w.ainsel().bits(0)
             .en().set_bit()
             .start_many().clear_bit()
        });
        
        // Unmask the specific ADC interrupt in the Nested Vector Interrupt Controller (NVIC).
        // This allows the CPU to actually jump to `ADC_IRQ_FIFO` when the event occurs.
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::ADC_IRQ_FIFO);
    }

    // 3. USB Serial Setup (CDC Class)
    // ----------------------------------------------------------------
    let usb_bus = hal::usb::UsbBus::new(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    );
    let bus_allocator = usb_device::bus::UsbBusAllocator::new(usb_bus);
    let mut serial = SerialPort::new(&bus_allocator);
    
    // Define USB device descriptors (VID/PID, Manufacturer, etc.)
    let mut usb_dev = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Raspberry Pi")
            .product("Pico 2 W ADC Demo")
            .serial_number("ADC001")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    // 4. GPIO Setup & FSM Init
    // ----------------------------------------------------------------
    // Configure the onboard LED. On Pico 2 W, this is typically GPIO15.
    let led_pin = pins.gpio15.into_push_pull_output();
    let mut last_toggle = timer.get_counter();

    // Initialize FSM
    let mut fsm_ctx = BlinkyContext { led: led_pin };
    let mut fsm = BlinkyFsm::LedOff; // Start in Off state
    fsm.init(&mut fsm_ctx); // Runs entry action for LedOff (sets pin low)

    // Send a welcome message via Serial (Best effort, might be missed if terminal isn't ready)
    let _ = serial.write(b"\r\n=== Pico 2 W ADC Demo (FSM Edition) ===\r\n");

    // 5. Main Application Loop
    // ----------------------------------------------------------------
    loop {
        // A. Poll USB
        // USB requires frequent polling (every few ms) to handle packets/enumerations.
        if usb_dev.poll(&mut [&mut serial]) {
            let mut buf = [0u8; 64];
            // Discard read data, just clearing the buffer
            let _ = serial.read(&mut buf);
        }

        // B. Timed Tasks (FSM Tick)
        let current_time = timer.get_counter();
        // Check if 200ms (200,000us) have passed
        if current_time.ticks().saturating_sub(last_toggle.ticks()) >= 200_000 {
            last_toggle = current_time;
            // Dispatch event to FSM. This will toggle LED and trigger ADC if turning ON.
            fsm.dispatch(&mut fsm_ctx, &BlinkyEvent::TimerTick);
        }

        // C. Process Data (Consumer)
        // Enter a Critical Section to safely access the shared global variables.
        // This disables interrupts temporarily to prevent race conditions.
        critical_section::with(|cs| {
            let mut ready_ref = ADC_READY.borrow_ref_mut(cs);
            
            if *ready_ref {
                // Consume the value
                let value = *ADC_VALUE.borrow_ref(cs);
                *ready_ref = false; // Reset flag

                // Format and send the message
                let mut msg: String<64> = String::new();
                // Convert raw 12-bit value (0-4095) to millivolts (0-3300mV)
                let mv = (value as u32 * 3300) / 4095;
                
                if FmtWrite::write_fmt(&mut msg, format_args!("ADC: {} ({}mV)\r\n", value, mv)).is_ok() {
                     let _ = serial.write(msg.as_bytes());
                }
            }
        });
    }
}

// --- Interrupt Handlers ---

/// ADC FIFO Interrupt Handler.
///
/// This function is called automatically by the hardware (NVIC) when the ADC FIFO
/// threshold is met (configured to 1 sample above).
#[allow(non_snake_case)]
#[interrupt]
fn ADC_IRQ_FIFO() {
    unsafe {
        let adc_regs = &(*pac::ADC::ptr());
        
        // Verify we actually have data in the FIFO
        if adc_regs.fcs().read().level().bits() > 0 {
            // Read the raw result from the FIFO register
            let value = adc_regs.fifo().read().val().bits();
            
            // Update shared state within a Critical Section
            critical_section::with(|cs| {
                *ADC_VALUE.borrow_ref_mut(cs) = value;
                *ADC_READY.borrow_ref_mut(cs) = true;
            });
        }
        // Note: The hardware interrupt flag is cleared automatically when the FIFO is drained.
    }
}

// --- Metadata ---

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 4] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Blinky Example"),
    hal::binary_info::rp_program_build_attribute!(),
];
