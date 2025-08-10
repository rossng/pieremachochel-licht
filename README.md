# PM-Licht LED Controller

A simple Rust application to control WS2812/NeoPixel LED strips connected to Raspberry Pi GPIO pins with a circus theme.

## Prerequisites

- Raspberry Pi 4 Model B (or compatible)
- WS2812/NeoPixel LED strip
- Proper power supply for LEDs (12V, sufficient current)

## Hardware Setup

1. Connect NeoPixel data line to GPIO18 (default) or specify another PWM-capable pin
2. Connect NeoPixel ground to Raspberry Pi ground
3. Connect NeoPixel power to external 5V power supply (do NOT power from Pi's 5V pin for strips with many LEDs)
4. Share ground between power supply and Raspberry Pi

## Setup

```
sudo apt install libclang-dev
sudo apt install socat
```

## Running

The application must be run with sudo due to GPIO access requirements:

```bash
cargo build --release && sudo ./target/release/pm-licht
```

## Options

- `--big-leds`: Use color profile for big LEDs instead of small LEDs
- `--mode-duration-secs <SECONDS>`: Time between mode switches (default: 30)

## Socket Control

Control animation speed via Unix socket at `/tmp/pm-licht`:

```bash
# Set speed to 1.5x (180 BPM from default 120 BPM)
echo '{ "command": ["set_property", "speed", 1.5] }' | socat - /tmp/pm-licht

# Set speed to 0.5x (60 BPM)
echo '{ "command": ["set_property", "speed", 0.5] }' | socat - /tmp/pm-licht
```