# NeoPixel Controller for Raspberry Pi

A Rust application to control WS2812/NeoPixel LED strips connected to Raspberry Pi GPIO pins.

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

## Building

```bash
cd neopixel-controller
cargo build --release
```

## Running

The application must be run with sudo due to GPIO access requirements:

```bash
sudo ./target/release/neopixel-controller [OPTIONS] <COMMAND>
```

## Commands

### Rainbow Animation
```bash
sudo ./target/release/neopixel-controller rainbow
sudo ./target/release/neopixel-controller rainbow --delay-ms 50
```

### Solid Color
```bash
sudo ./target/release/neopixel-controller solid --red 255 --green 0 --blue 0
sudo ./target/release/neopixel-controller solid -r 0 -g 255 -b 0
```

### Chase Animation
```bash
sudo ./target/release/neopixel-controller chase --red 255 --green 255 --blue 255
sudo ./target/release/neopixel-controller chase -r 255 -g 0 -b 255 --delay-ms 30
```

### Breathe Animation
```bash
sudo ./target/release/neopixel-controller breathe
sudo ./target/release/neopixel-controller breathe -r 255 -g 100 -b 0 --delay-ms 5
```

### Turn Off
```bash
sudo ./target/release/neopixel-controller off
```

## Global Options

- `-n, --num-leds <NUM_LEDS>`: Number of LEDs in the strip (default: 60)
- `-g, --gpio-pin <GPIO_PIN>`: GPIO pin number (default: 18)
- `-b, --brightness <BRIGHTNESS>`: Global brightness 0-255 (default: 255)

## Examples

```bash
# 30 LED strip on GPIO 12 with rainbow effect
sudo ./target/release/neopixel-controller -n 30 -g 12 rainbow

# Bright red solid color with 50% brightness
sudo ./target/release/neopixel-controller -b 128 solid -r 255 -g 0 -b 0

# Fast blue chase on 144 LED strip
sudo ./target/release/neopixel-controller -n 144 chase -r 0 -g 0 -b 255 --delay-ms 20
```

## Troubleshooting

1. **Permission denied**: Make sure to run with `sudo`
2. **LEDs not lighting**: Check power connections and ensure ground is shared
3. **Wrong colors**: Some strips use GRB instead of RGB order (modify StripType in code)
4. **Flickering**: May need a level shifter for 3.3V to 5V conversion

## Cross-compilation (from development machine to Pi)

If developing on a non-Pi machine, install cross-compilation tools:

```bash
# Install cross-compilation target
rustup target add armv7-unknown-linux-gnueabihf

# Build for Pi
cargo build --release --target armv7-unknown-linux-gnueabihf

# Copy to Pi
scp target/armv7-unknown-linux-gnueabihf/release/neopixel-controller pi@raspberrypi.local:~
```