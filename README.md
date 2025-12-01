# Typed FSM on Raspberry Pi Pico 2 W (RP2350)

This project is a **proof-of-concept** for using the [typed-fsm](https://crates.io/crates/typed-fsm) crate in an embedded Rust environment (`no_std`).

It demonstrates a robust, interrupt-driven architecture running on the **Raspberry Pi Pico 2 W (RP2350)**, featuring:
- **Typed State Machine**: Declarative logic controlling the application flow.
- **Modular Architecture**: Clean separation between Hardware, USB, and Logic.
- **Interrupts**: ADC and USB handling via hardware interrupts.
- **Thread Safety**: Safe data exchange using Critical Sections and Mutexes.

---

## Application Behavior

The system implements a "Blinky" state machine with a safety lockout mechanism based on ADC readings.

### States
1.  **OFF**: LED is off. Waiting for timer tick.
2.  **ON**: LED is on. ADC is triggered automatically.
3.  **WAIT_HIGH_VALUE**: Safety lockout state. LED is off. System waits for 2 seconds before resetting.

### Logic Flow
1.  **Timer Trigger**: Every 200ms, a timer event is sent to the FSM.
    *   If in **OFF**, it transitions to **ON**.
    *   If in **ON**, it transitions back to **OFF**.
2.  **ADC Trigger**: When entering **ON** state, an ADC conversion (Ch0/GPIO26) is triggered.
3.  **Safety Check**: When ADC conversion completes (ISR), the value is checked:
    *   If **Value > 70**: Transition to **WAIT_HIGH_VALUE**.
    *   If **Value <= 70**: Continue normal operation.
4.  **Cooldown**: In **WAIT_HIGH_VALUE**, the system ignores ADC readings and waits for ~2 seconds (10 ticks) before returning to **OFF**, provided the ADC value has returned to a safe level (<= 70).

---

## Hardware Setup

| Pin | Function | Description |
|-----|----------|-------------|
| **GPIO15** | Output | **LED**. Blinks to indicate state. (Built-in on Pico 2 W) |
| **GPIO26** | Input | **ADC Ch0**. Connect a potentiometer or sensor (0-3.3V). |
| **USB** | Serial | **Telemetry**. Connect to PC to view state logs. |

---

## Development Environment Setup

To replicate this environment and develop effectively, follow these steps.

### 1. IDE & Tools
We recommend **Visual Studio Code** (VS Code) for the best experience.

**Recommended Extensions:**
- **[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)**: Essential for code completion, type inference, and inline errors.
- **[Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)**: For editing `Cargo.toml` files.
- **[Dependi](https://marketplace.visualstudio.com/items?itemName=fill-labs.dependi)**: Helps manage Rust dependencies.
- **[Raspberry Pi Pico](https://marketplace.visualstudio.com/items?itemName=raspberry-pi.raspberry-pi-pico)**: Official extension for Raspberry Pi Pico development, including project creation, flashing, and debugging setup.

### 2. Rust Toolchain
Install Rust via [rustup](https://rustup.rs/).

Add the target architecture for the RP2350 (Cortex-M33):
```bash
rustup target add thumbv8m.main-none-eabihf
```

### 3. Helper Tools
Install these cargo subcommands to facilitate building and flashing:

- **elf2uf2-rs**: Converts the compiled ELF binary to UF2 format for drag-and-drop flashing.
  ```bash
  cargo install elf2uf2-rs
  ```
- **picotool**: Official Raspberry Pi tool for inspecting and flashing binaries (optional but useful).
  (Install via your system package manager, e.g., `brew install picotool` on macOS)

---

## How to Run

### 1. Build & Flash Automatically
This project includes a configured runner script. Connect your Pico 2 W while holding the **BOOTSEL** button (so it mounts as a drive).

```bash
cargo run --release
```
*This command will compile the code, automatically convert it to a `.uf2` file, and attempt to copy it to the mounted RP2350 drive.*

### 2. Manual Flashing (Fallback)
If the automatic runner fails:
1.  Run `cargo build --release`.
2.  Run `elf2uf2-rs target/thumbv8m.main-none-eabihf/release/tst_rust target/thumbv8m.main-none-eabihf/release/tst_rust.uf2`.
3.  Drag and drop the generated `.uf2` file to the `RP2350` volume.

### 3. Monitor Output
Open a serial terminal to view the FSM state transitions in real-time.

```bash
# macOS / Linux
screen /dev/cu.usbmodem* 115200

# Windows (using PuTTY or similar)
# Connect to the assigned COM port
# Baud rate is ignored by USB CDC, but 115200 is standard.
```

**Expected Output:**
```text
=== Pico 2 W ADC Demo (Shared FSM) ===
State: ON
State: OFF
State: ON
...
(If ADC > 70)
State: WAIT_HIGH_VALUE
State: WAIT_HIGH_VALUE
...
(After 2s and ADC <= 70)
State: OFF
```

---

## Project Structure

This project follows a modular architecture to keep `main.rs` clean and readable.

- **`src/main.rs`**: The application entry point. It orchestrates the initialization, the main loop, and the high-level FSM dispatching.
- **`src/blinky_fsm.rs`**: Defines the **Finite State Machine** definition using `typed-fsm`. It defines the States (`LedOff`, `LedOn`, `HighValueWait`), Events, and Transitions.
- **`src/hardware.rs`**: Abstraction layer for **Hardware Configuration**. It sets up the Clocks, PLLs, Timer, GPIOs, and ADC, returning a struct with ready-to-use peripherals.
- **`src/usb_module.rs`**: Encapsulates the **USB Serial Stack**. It handles global static variables, initialization, and the USB Interrupt Service Routine (ISR).

## License
MIT or Apache-2.0.