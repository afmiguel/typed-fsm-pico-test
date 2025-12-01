# ADC Interrupt Demo for Raspberry Pi Pico 2 W

This project demonstrates interrupt-driven ADC sampling on the RP2350 (Raspberry Pi Pico 2 W) with USB Serial output.

## Features

- **Non-blocking ADC trigger**: ADC conversion is triggered when GPIO15 goes HIGH
- **Interrupt-driven reading**: ADC value is read automatically in the ISR (Interrupt Service Routine)
- **USB Serial output**: ADC readings sent to computer via USB CDC
- **Dual architecture support**: Works on both ARM (Cortex-M33) and RISC-V (Hazard3)

## Hardware Setup

| Pin | Function | Description |
|-----|----------|-------------|
| **GPIO15** | Digital Output | LED / Trigger (toggles every 200ms) |
| **GPIO26** | ADC Input | Analog input (Channel 0) - 0-3.3V |
| **USB** | Serial CDC | Communication with computer |

**Note**: GPIO25 is NOT available on Pico 2 W (used by WiFi module). We use GPIO15 instead.

### Connecting ADC Input

Connect to GPIO26:
- **Option 1**: Potentiometer between 3.3V and GND
- **Option 2**: Voltage divider (max 3.3V!)
- **Option 3**: Sensor output (0-3.3V range)

## Building the Project

### Prerequisites

- Rust toolchain (install from https://rustup.rs)
- ARM target: `rustup target add thumbv8m.main-none-eabihf`
- RISC-V target: `rustup target add riscv32imac-unknown-none-elf`
- `elf2uf2-rs`: `cargo install elf2uf2-rs`
- `picotool`: Install from https://github.com/raspberrypi/picotool

### Compile

```bash
cargo build --release
```

This generates:
- **ELF file**: `target/thumbv8m.main-none-eabihf/release/tst_rust`
- **UF2 file**: `target/thumbv8m.main-none-eabihf/release/tst_rust.uf2` (via `cargo run`)

## Flashing the Firmware

### Method 1: Using `cargo run` (Automatic UF2 generation + Flash)

1. **Put Pico in BOOTSEL mode**:
   - Disconnect USB cable
   - Hold down **BOOTSEL** button on Pico
   - Connect USB cable (while holding BOOTSEL)
   - Release BOOTSEL button
   - Pico will appear as USB drive: `/Volumes/RP2350`

2. **Flash automatically**:
   ```bash
   cargo run --release
   ```

   This will:
   - Compile the code
   - Generate UF2 file with correct family ID (rp2350-arm-s)
   - Copy to Pico
   - Sync and eject
   - Pico will reboot automatically

3. **Manually eject if needed**:
   - If Pico doesn't reboot automatically
   - Open Finder and eject "RP2350" drive
   - **OR** drag RP2350 icon to Trash

### Method 2: Manual UF2 Copy

1. **Compile and generate UF2**:
   ```bash
   cargo build --release
   cargo run --release  # Generates .uf2 file
   ```

2. **Put Pico in BOOTSEL mode** (see Method 1)

3. **Copy UF2 file**:
   ```bash
   cp target/thumbv8m.main-none-eabihf/release/tst_rust.uf2 /Volumes/RP2350/
   ```

4. **Eject drive**:
   ```bash
   sync
   diskutil eject /Volumes/RP2350
   ```

### Method 3: Using Makefile

```bash
# Compile + generate UF2
make

# Flash via bootloader (Pico must be in BOOTSEL mode)
make flash-uf2

# View serial monitor
make monitor
```

## Viewing Serial Output

After flashing, the Pico will appear as a USB serial device.

### Find the serial port

```bash
ls /dev/cu.usbmodem*
```

You should see: `/dev/cu.usbmodemADC0011`

### Connect with screen

```bash
screen /dev/cu.usbmodemADC0011 115200
```

**To exit screen**: Press `Ctrl+A`, then `K`, then `Y`

### Expected Output

```
=== Pico 2 W ADC Demo ===
GPIO26 ADC readings will appear below

ADC: 2048 (1650mV)
ADC: 2051 (1652mV)
ADC: 2049 (1651mV)
...
```

## How It Works

1. **GPIO15** goes HIGH (every 200ms)
2. **ADC conversion** is triggered on GPIO26 (single-shot)
3. **~2µs later**: ADC completes, triggers interrupt `ADC_IRQ_FIFO`
4. **ISR executes**: Reads value from FIFO, stores in `ADC_VALUE`
5. **Main loop**: Detects new value, sends via USB Serial
6. **You see**: `ADC: 2048 (1650mV)` on your computer

## Project Structure

```
.
├── src/
│   └── main.rs           # Main application code
├── .cargo/
│   ├── config.toml       # Cargo build configuration
│   └── runner.sh         # Custom flash script
├── Cargo.toml            # Dependencies
├── Makefile              # Build automation
├── build.rs              # Build script
└── README.md             # This file
```

## Troubleshooting

### Pico doesn't reboot after flash

- **Manually eject** the RP2350 drive from Finder
- Or run: `diskutil eject /Volumes/RP2350`

### No serial port appears

- Check USB cable (must support data, not just power)
- Try different USB port
- Reconnect Pico

### Wrong Family ID error

If you see `Family ID 'rp2040' cannot be downloaded`:
- The UF2 was generated for RP2040 instead of RP2350
- Use: `picotool uf2 convert` with `--family rp2350-arm-s`
- Or use `cargo run` which does this automatically

### ADC reads 0 or 4095

- **GPIO26 floating**: Not connected to anything
- **GPIO26 shorted**: Check connections
- **Solution**: Connect a voltage source (potentiometer, sensor)

### Compilation errors

```bash
# Clean and rebuild
cargo clean
cargo build --release
```

## Technical Details

### Interrupt Handling

- **ARM (Cortex-M33)**: Uses NVIC (Nested Vectored Interrupt Controller)
  - `cortex_m::peripheral::NVIC::unmask()`
- **RISC-V (Hazard3)**: Uses custom interrupt controller (not standard PLIC)
  - `riscv::register::mie::set_mext()` - Enable external interrupts
  - `riscv::interrupt::enable()` - Enable global interrupts

### ADC Specifications

- **Resolution**: 12-bit (0-4095)
- **Input channels**: GPIO26, GPIO27, GPIO28, GPIO29
- **Conversion time**: ~2µs (96 ADC clock cycles)
- **Voltage range**: 0-3.3V (DO NOT EXCEED!)

### USB Serial

- **Class**: CDC (Communications Device Class)
- **VID:PID**: 0x16c0:0x27dd
- **Product name**: "Pico 2 W ADC Demo"
- **Baud rate**: 115200 (actually ignored for USB CDC)

## License

MIT OR Apache-2.0

## Credits

Based on [rp-hal](https://github.com/rp-rs/rp-hal) examples.

Copyright (c) 2021–2024 The rp-rs Developers
Copyright (c) 2025 Raspberry Pi Ltd.
