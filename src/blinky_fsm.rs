use embedded_hal::digital::OutputPin;
use typed_fsm::{state_machine, Transition};

use rp235x_hal as hal;

pub type LedPin = hal::gpio::Pin<
    hal::gpio::bank0::Gpio15,
    hal::gpio::FunctionSio<hal::gpio::SioOutput>,
    hal::gpio::PullDown,
>;

// Helper function to trigger ADC
fn trigger_adc() {
    unsafe {
        let adc_regs = &(*hal::pac::ADC::ptr());
        adc_regs.cs().modify(|_, w| w.start_once().set_bit());
    }
}

// FSM Context
pub struct BlinkyContext {
    pub led: LedPin,
    pub wait_ticks: u32, // Counter for the wait state
    pub last_adc_value: u16, // Stores the last ADC value received
}

// FSM Events
#[derive(Clone, Copy, Debug)]
pub enum BlinkyEvent {
    TimerTick,
    AdcResult(u16),
}

// State Machine Definition
state_machine! {
    Name: BlinkyFsm,
    Context: BlinkyContext,
    Event: BlinkyEvent,
    States: {
        // State: LED Off
        LedOff => {
            entry: |ctx| {
                let _ = ctx.led.set_low();
            }
            process: |_ctx, evt| {
                match evt {
                    BlinkyEvent::TimerTick => Transition::To(BlinkyFsm::LedOn),
                    BlinkyEvent::AdcResult(_) => Transition::None, // Ignored in this state
                }
            }
        },

        // State: LED On
        LedOn => {
            entry: |ctx| {
                let _ = ctx.led.set_high();
                trigger_adc();
            }
            process: |_ctx, evt| {
                match evt {
                    BlinkyEvent::TimerTick => Transition::To(BlinkyFsm::LedOff),
                    BlinkyEvent::AdcResult(val) => {
                        if *val > 70 {
                            Transition::To(BlinkyFsm::HighValueWait)
                        } else {
                            Transition::None // ADC value okay, no state change
                        }
                    }
                }
            }
        },

        // State: High Value Wait (Cooldown)
        HighValueWait => {
            entry: |ctx| {
                let _ = ctx.led.set_low(); // Turn off LED to indicate waiting
                ctx.wait_ticks = 0;       // Reset counter
            }
            process: |ctx, evt| {
                match evt {
                    BlinkyEvent::TimerTick => {
                        trigger_adc(); // Trigger ADC to get a new value for exit condition
                        ctx.wait_ticks += 1;
                        // Assuming TimerTick happens every 200ms.
                        // 2 seconds / 200ms = 10 ticks.
                        if ctx.wait_ticks >= 10 && ctx.last_adc_value <= 70 {
                            Transition::To(BlinkyFsm::LedOff) // Time up AND ADC value is safe
                        } else {
                            Transition::None // Keep waiting
                        }
                    },
                    BlinkyEvent::AdcResult(val) => {
                        ctx.last_adc_value = *val; // Update last known ADC value
                        Transition::None // Stay in this state
                    },
                }
            }
        }
    }
}