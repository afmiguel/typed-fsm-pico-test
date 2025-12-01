//! SPDX-License-Identifier: MIT OR Apache-2.0
//!
//! # GPIO 'Blinky' with ADC Interrupt and USB Serial Example
//!
//! This application demonstrates a professional embedded Rust architecture using:
//! - **Hardware Module:** Clean abstraction for HAL setup (`hardware.rs`).
//! - **USB Module:** Isolated USB stack with interrupts (`usb_module.rs`).
//! - **FSM:** Typed state machine for application logic (`blinky_fsm.rs`).
//!
//! Target: Raspberry Pi Pico 2 W (RP2350).

#![no_std]
#![no_main]

// --- Imports ---
use core::cell::RefCell;
use core::fmt::Write as FmtWrite;
use critical_section::Mutex;
use defmt ::*;
use defmt_rtt as _;
use heapless::String;
use panic_probe as _;

// --- Modules ---
mod blinky_fsm;
use blinky_fsm::{BlinkyContext, BlinkyEvent, BlinkyFsm};

mod usb_module;
mod hardware;

// --- HAL Selection ---
use rp235x_hal as hal;
use hal::entry;
use hal::pac;

// Select appropriate interrupt macro based on chip architecture
use rp235x_hal::pac::interrupt;

// --- Bootloader Configuration ---

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

// --- Shared State ---

// Global FSM and Context
static GLOBAL_FSM: Mutex<RefCell<Option<BlinkyFsm>>> = Mutex::new(RefCell::new(None));
static GLOBAL_CTX: Mutex<RefCell<Option<BlinkyContext>>> = Mutex::new(RefCell::new(None));

/// Entry point.
#[entry]
fn main() -> ! {
    info!("Program start");

    // 1. Initialize Hardware Stack (Clocks, GPIO, Timer, ADC, USB)
    let hw = hardware::init();

    // 2. Initialize Application State (FSM)
    let mut fsm_ctx = BlinkyContext { led: hw.led_pin, wait_ticks: 0, last_adc_value: 0 };
    let mut fsm = BlinkyFsm::LedOff;
    fsm.init(&mut fsm_ctx);

    // 3. Publish FSM to Global State (for ISR access)
    critical_section::with(|cs| {
        GLOBAL_FSM.borrow_ref_mut(cs).replace(fsm);
        GLOBAL_CTX.borrow_ref_mut(cs).replace(fsm_ctx);
    });

    let mut last_toggle = hw.timer.get_counter();

    // 4. Main Application Loop
    loop {
        let current_time = hw.timer.get_counter();
        
        // Periodic Task: Timer Tick (every 200ms)
        if current_time.ticks().saturating_sub(last_toggle.ticks()) >= 200_000 {
            last_toggle = current_time;
            
            let mut current_state_str = "Unknown";

            // Dispatch TimerTick Event directly to global FSM
            critical_section::with(|cs| {
                let mut fsm_guard = GLOBAL_FSM.borrow_ref_mut(cs);
                let mut ctx_guard = GLOBAL_CTX.borrow_ref_mut(cs);
                
                if let (Some(fsm), Some(ctx)) = (fsm_guard.as_mut(), ctx_guard.as_mut()) {
                    fsm.dispatch(ctx, &BlinkyEvent::TimerTick);
                    
                    match fsm {
                        BlinkyFsm::LedOff => current_state_str = "OFF",
                        BlinkyFsm::LedOn => current_state_str = "ON",
                        BlinkyFsm::HighValueWait => current_state_str = "WAIT_HIGH_VALUE",
                    }
                }
            });

            // Log state to USB
            let mut msg: String<64> = String::new();
            if FmtWrite::write_fmt(&mut msg, format_args!("State: {}\r\n", current_state_str)).is_ok() {
                usb_module::write(msg.as_bytes());
            }
        }
    }
}

// --- Interrupt Handlers ---

#[allow(non_snake_case)]
#[interrupt]
fn ADC_IRQ_FIFO() {
    unsafe {
        let adc_regs = &(*pac::ADC::ptr());
        
        if adc_regs.fcs().read().level().bits() > 0 {
            let value = adc_regs.fifo().read().val().bits();
            
            // Dispatch AdcResult Event directly to global FSM
            critical_section::with(|cs| {
                let mut fsm_guard = GLOBAL_FSM.borrow_ref_mut(cs);
                let mut ctx_guard = GLOBAL_CTX.borrow_ref_mut(cs);
                
                if let (Some(fsm), Some(ctx)) = (fsm_guard.as_mut(), ctx_guard.as_mut()) {
                    fsm.dispatch(ctx, &BlinkyEvent::AdcResult(value as u16));
                }
            });
        }
    }
}

// --- Metadata ---

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 4] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Blinky Example"),
    hal::binary_info::rp_program_build_attribute!()
];
