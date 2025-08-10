# PM-Licht LED Controller

A simple Rust application to control WS2812/NeoPixel LED strips connected to Raspberry Pi GPIO pins with a chase animation.

## Prerequisites

- Raspberry Pi 4 Model B (or compatible)
- WS2812/NeoPixel LED strip
- Proper power supply for LEDs (5V, sufficient current)
- Level shifter (recommended for 3.3V to 5V signal conversion)

## Hardware Setup

1. Connect NeoPixel data line to GPIO18 (default) or specify another PWM-capable pin
2. Connect NeoPixel ground to Raspberry Pi ground
3. Connect NeoPixel power to external 5V power supply (do NOT power from Pi's 5V pin for strips with many LEDs)
4. Share ground between power supply and Raspberry Pi

## Setup

```
sudo apt install libclang-dev
```

## Running

The application must be run with sudo due to GPIO access requirements:

```bash
cargo build --release && sudo ./target/release/pm-licht
```