//! USB Module
//!
//! This module encapsulates the USB Serial handling.
//! It manages the global static resources required for the USB stack,
//! handles the initialization, and implements the `USBCTRL_IRQ` interrupt handler
//! to ensure robust communication.

use core::cell::RefCell;
use critical_section::Mutex;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

use rp235x_hal as hal;
use hal::pac;

// Select appropriate interrupt macro based on chip architecture
use rp235x_hal::pac::interrupt;

type UsbBusType = hal::usb::UsbBus;

// Global USB Objects (Mutex protected for ISR access)
static USB_DEVICE: Mutex<RefCell<Option<UsbDevice<UsbBusType>>>> = Mutex::new(RefCell::new(None));
static USB_SERIAL: Mutex<RefCell<Option<SerialPort<UsbBusType>>>> = Mutex::new(RefCell::new(None));

/// Initialize USB Serial and enable the USB interrupt.
///
/// This setup includes creating the static bus allocator using `unsafe` (safe pattern for no_std),
/// initializing the SerialPort and UsbDevice, and unmasking the NVIC interrupt.
pub fn init(
    usb_periph: pac::USB,
    usb_dpram: pac::USB_DPRAM,
    usb_clock: hal::clocks::UsbClock,
    resets: &mut pac::RESETS,
) {
    // 1. Create the USB Bus
    let usb_bus = hal::usb::UsbBus::new(
        usb_periph,
        usb_dpram,
        usb_clock,
        true,
        resets,
    );
    
    // 2. Create static allocator (safe pattern for no_std)
    // We use a static variable to ensure the allocator lives for the entire program duration.
    static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None;
    
    // Safety: This is called only once at initialization time, before interrupts are enabled.
    let bus_allocator = unsafe {
        let bus_ptr = core::ptr::addr_of_mut!(USB_BUS);
        *bus_ptr = Some(usb_device::bus::UsbBusAllocator::new(usb_bus));
        (*bus_ptr).as_ref().unwrap()
    };

    // 3. Create Device and Serial Port
    let serial = SerialPort::new(bus_allocator);
    let usb_dev = UsbDeviceBuilder::new(bus_allocator, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Raspberry Pi")
            .product("Pico 2 W ADC Demo")
            .serial_number("ADC001")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    // 4. Move to Global Storage
    critical_section::with(|cs| {
        USB_DEVICE.borrow_ref_mut(cs).replace(usb_dev);
        USB_SERIAL.borrow_ref_mut(cs).replace(serial);
    });

    // 5. Enable Interrupt
    unsafe {
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
    }
}

/// Write data to the USB Serial port safely.
///
/// This function uses a critical section to access the global `USB_SERIAL` object
/// and write bytes to the host.
pub fn write(data: &[u8]) {
    critical_section::with(|cs| {
        let mut serial = USB_SERIAL.borrow_ref_mut(cs);
        if let Some(serial) = serial.as_mut() {
            let _ = serial.write(data);
        }
    });
}

/// USB Interrupt Handler
///
/// Handles all USB events (Enumeration, Data In/Out) automatically.
/// By using an interrupt, we ensure the USB connection remains stable
/// even if the main loop is busy.
#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    critical_section::with(|cs| {
        let mut dev = USB_DEVICE.borrow_ref_mut(cs);
        let mut serial = USB_SERIAL.borrow_ref_mut(cs);

        if let (Some(dev), Some(serial)) = (dev.as_mut(), serial.as_mut()) {
            if dev.poll(&mut [serial]) {
                // Consume data to clear buffer (echo or logic could be added here)
                let mut buf = [0u8; 64];
                let _ = serial.read(&mut buf);
            }
        }
    });
}
