use embedded_hal::digital::OutputPin;
use typed_fsm::{state_machine, Transition};

#[cfg(rp2350)]
use rp235x_hal as hal;
#[cfg(rp2040)]
use rp2040_hal as hal;

// Define LedPin type alias for both architectures
#[cfg(any(rp2040, rp2350))]
pub type LedPin = hal::gpio::Pin<
    hal::gpio::bank0::Gpio15,
    hal::gpio::FunctionSio<hal::gpio::SioOutput>,
    hal::gpio::PullDown,
>;

// Helper function to trigger ADC
// We use direct register access to avoid passing the ADC peripheral around
fn trigger_adc() {
    unsafe {
        let adc_regs = &(*hal::pac::ADC::ptr());
        adc_regs.cs().modify(|_, w| w.start_once().set_bit());
    }
}

// FSM Context
pub struct BlinkyContext {
    pub led: LedPin,
}

// FSM Events
pub enum BlinkyEvent {
    TimerTick,
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
                }
            }
        }
    }
}