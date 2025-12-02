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

/// Wrapper struct to unify FSM and Context into a single global resource.
/// This reduces locking overhead (one Mutex vs two) and simplifies data management.
pub struct AppState {
    fsm: BlinkyFsm,
    ctx: BlinkyContext,
}

// Global Application State (Mutex protected for ISR access)
static GLOBAL_STATE: Mutex<RefCell<Option<AppState>>> = Mutex::new(RefCell::new(None));

/// Entry point.
#[entry]
fn main() -> ! {
    info!("Program start");

    // 1. Initialize Hardware Stack (Clocks, GPIO, Timer, ADC, USB)
    // Now returns a tuple directly, removing the ephemeral `Hardware` struct.
    let (led_pin, timer) = hardware::init();

    // 2. Initialize Application State (FSM)
    // Create context and initial state immediately.
    let mut ctx = BlinkyContext { 
        led: led_pin, 
        wait_ticks: 0, 
        last_adc_value: 0 
    };
    let mut fsm = BlinkyFsm::LedOff;
    fsm.init(&mut ctx);

    // 3. Publish to Global State
    // Bundled initialization eliminates the "dance" of separate Options.
    critical_section::with(|cs| {
        GLOBAL_STATE.borrow_ref_mut(cs).replace(AppState { fsm, ctx });
    });

    let mut last_toggle = timer.get_counter();

    // 4. Main Application Loop
    loop {
        let current_time = timer.get_counter();
        
        // Periodic Task: Timer Tick (every 200ms)
        if current_time.ticks().saturating_sub(last_toggle.ticks()) >= 200_000 {
            last_toggle = current_time;
            
            let mut current_state_str = "Unknown";

            // Dispatch TimerTick Event using unified global state
            critical_section::with(|cs| {
                let mut state_guard = GLOBAL_STATE.borrow_ref_mut(cs);
                
                if let Some(state) = state_guard.as_mut() {
                    state.fsm.dispatch(&mut state.ctx, &BlinkyEvent::TimerTick);
                    
                    match state.fsm {
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
            
            // Dispatch AdcResult Event using unified global state
            critical_section::with(|cs| {
                let mut state_guard = GLOBAL_STATE.borrow_ref_mut(cs);
                
                if let Some(state) = state_guard.as_mut() {
                    state.fsm.dispatch(&mut state.ctx, &BlinkyEvent::AdcResult(value as u16));
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