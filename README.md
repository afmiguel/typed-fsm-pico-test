# Typed FSM on Raspberry Pi Pico 2 W (RP2350)

This project is a **proof-of-concept** for using the [typed-fsm](https://crates.io/crates/typed-fsm) crate in an embedded Rust environment (`no_std`).

It demonstrates a robust, interrupt-driven architecture running on the **Raspberry Pi Pico 2 W (RP2350)**, featuring:
- **Typed State Machine**: Declarative logic controlling the application flow.
- **Modular Architecture**: Clean separation between Hardware, USB, and Logic.
- **Interrupts**: ADC and USB handling via hardware interrupts.
- **Thread Safety**: Safe data exchange using Critical Sections and Mutexes (Unified Global State).

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
4.  **Cooldown**: In **WAIT_HIGH_VALUE**, the system triggers ADC readings on every tick and waits for ~2 seconds (10 ticks). It only returns to **OFF** if the cooldown is complete AND the ADC value is safe (<= 70).

---

## Hardware Setup

| Pin | Function | Description |
|-----|----------|-------------|
| **GPIO15** | Output | **LED**. Blinks to indicate state. (Built-in on Pico 2 W) |
| **GPIO26** | Input | **ADC Ch0**. Connect a potentiometer or sensor (0-3.3V). |
| **USB** | Serial | **Telemetry**. Connect to PC to view state logs. |

---

## Development Environment Setup

To build and flash this project, you need to set up the Rust toolchain and platform-specific tools.

### 1. Install Rust
Install Rust via [rustup](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
*On Windows, download and run `rustup-init.exe`.*

### 2. Add Target Architecture
The RP2350 uses the Arm Cortex-M33 architecture.
```bash
rustup target add thumbv8m.main-none-eabihf
```

### 3. Platform Specific Setup

#### 🍎 macOS
1.  **Install Dependencies**:
    ```bash
    brew install picotool
    ```
2.  **Install Cargo Tools**:
    ```bash
    cargo install elf2uf2-rs --locked
    cargo install flip-link
    ```

#### 🐧 Linux (Ubuntu/Debian)
1.  **Install System Dependencies**:
    ```bash
    sudo apt update
    sudo apt install -y git build-essential libudev-dev libusb-1.0-0-dev pkg-config
    ```
2.  **Install Picotool** (from source if not in repo, or use package manager):
    *Refer to official Raspberry Pi documentation for building picotool on Linux if needed.*
3.  **Install Cargo Tools**:
    ```bash
    cargo install elf2uf2-rs --locked
    cargo install flip-link
    ```
4.  **USB Permissions (udev rules)**:
    Create a file `/etc/udev/rules.d/99-pico.rules` with:
    ```text
    SUBSYSTEM=="usb", ATTRS{idVendor}=="2e8a", ATTRS{idProduct}=="000a", MODE="0666"
    ```
    Then reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`.

#### 🪟 Windows
1.  **Install Build Tools**: Ensure you have the "Desktop development with C++" workload installed via Visual Studio Build Tools.
2.  **Install Drivers**: In most cases, Windows 10/11 detects the Pico automatically. If not, use [Zadig](https://zadig.akeo.ie/) to install the WinUSB driver for the device.
3.  **Install Cargo Tools**:
    ```powershell
    cargo install elf2uf2-rs --locked
    cargo install flip-link
    ```

---

## How to Run

### 1. Build & Flash Automatically
This project is configured to use a runner script (or `elf2uf2-rs` depending on config).

1.  Connect your Pico 2 W while holding the **BOOTSEL** button (it should mount as a mass storage drive named `RP2350`).
2.  Run:
    ```bash
    cargo run --release
    ```
    *This command compiles the binary and copies the `.uf2` file to the device, which will reboot and start running.*

### 2. Monitor Output
Open a serial terminal to view the FSM state transitions.

- **macOS/Linux**: `minicom -D /dev/ttyACM0` or `screen /dev/cu.usbmodem... 115200`
- **Windows**: Use PuTTY or Serial Monitor in VS Code.

**Expected Output:**
```text
State: ON
State: OFF
State: ON
...
(If ADC > 70)
State: WAIT_HIGH_VALUE
...
```

---

## Project Structure

This project follows a modular architecture:

- **`src/main.rs`**: Entry point. Initializes hardware, sets up the **Unified Global State** (`AppState`), and runs the main loop.
- **`src/blinky_fsm.rs`**: Defines the **Finite State Machine** using `typed-fsm`. Contains States (`LedOff`, `LedOn`, `HighValueWait`), Events, and Transitions.
- **`src/hardware.rs`**: Abstraction layer. Initializes Clocks, Timer, GPIO, and ADC, returning a simple tuple of resources.
- **`src/usb_module.rs`**: Encapsulates the USB Serial stack and interrupt handling.

## License
MIT or Apache-2.0.
