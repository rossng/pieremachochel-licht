use anyhow::Result;
use clap::{Parser, Subcommand};
use rs_ws281x::{ChannelBuilder, ControllerBuilder, StripType};
use std::thread;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "neopixel-controller")]
#[command(about = "Control NeoPixels on Raspberry Pi GPIO")]
struct Cli {
    #[arg(short, long, default_value_t = 60)]
    num_leds: i32,

    #[arg(short, long, default_value_t = 18)]
    gpio_pin: i32,

    #[arg(short, long, default_value_t = 255)]
    brightness: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Rainbow {
        #[arg(short, long, default_value_t = 20)]
        delay_ms: u64,
    },
    Solid {
        #[arg(short = 'r', long, default_value_t = 255)]
        red: u8,
        #[arg(short = 'g', long, default_value_t = 0)]
        green: u8,
        #[arg(short = 'b', long, default_value_t = 0)]
        blue: u8,
    },
    Chase {
        #[arg(short = 'r', long, default_value_t = 255)]
        red: u8,
        #[arg(short = 'g', long, default_value_t = 0)]
        green: u8,
        #[arg(short = 'b', long, default_value_t = 0)]
        blue: u8,
        #[arg(short, long, default_value_t = 50)]
        delay_ms: u64,
    },
    Breathe {
        #[arg(short = 'r', long, default_value_t = 0)]
        red: u8,
        #[arg(short = 'g', long, default_value_t = 100)]
        green: u8,
        #[arg(short = 'b', long, default_value_t = 255)]
        blue: u8,
        #[arg(short, long, default_value_t = 10)]
        delay_ms: u64,
    },
    Off,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(10)
        .channel(
            0,
            ChannelBuilder::new()
                .pin(cli.gpio_pin)
                .count(cli.num_leds)
                .strip_type(StripType::Ws2812)
                .brightness(cli.brightness)
                .build(),
        )
        .build()?;

    match cli.command {
        Commands::Rainbow { delay_ms } => rainbow_animation(&mut controller, cli.num_leds, delay_ms)?,
        Commands::Solid { red, green, blue } => solid_color(&mut controller, cli.num_leds, red, green, blue)?,
        Commands::Chase { red, green, blue, delay_ms } => chase_animation(&mut controller, cli.num_leds, red, green, blue, delay_ms)?,
        Commands::Breathe { red, green, blue, delay_ms } => breathe_animation(&mut controller, cli.num_leds, red, green, blue, delay_ms)?,
        Commands::Off => turn_off(&mut controller, cli.num_leds)?,
    }

    Ok(())
}

fn rainbow_animation(controller: &mut rs_ws281x::Controller, num_leds: i32, delay_ms: u64) -> Result<()> {
    println!("Running rainbow animation...");
    let mut offset = 0;
    
    loop {
        for i in 0..num_leds {
            let hue = ((i + offset) * 360 / num_leds) as f32;
            let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
            controller.leds_mut(0)[i as usize] = [r, g, b, 0];
        }
        controller.render()?;
        thread::sleep(Duration::from_millis(delay_ms));
        offset = (offset + 1) % num_leds;
    }
}

fn solid_color(controller: &mut rs_ws281x::Controller, num_leds: i32, r: u8, g: u8, b: u8) -> Result<()> {
    println!("Setting solid color: R={}, G={}, B={}", r, g, b);
    
    for i in 0..num_leds {
        controller.leds_mut(0)[i as usize] = [r, g, b, 0];
    }
    controller.render()?;
    
    println!("Press Ctrl+C to exit");
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn chase_animation(controller: &mut rs_ws281x::Controller, num_leds: i32, r: u8, g: u8, b: u8, delay_ms: u64) -> Result<()> {
    println!("Running chase animation...");
    let mut position = 0;
    
    loop {
        for i in 0..num_leds {
            if i == position {
                controller.leds_mut(0)[i as usize] = [r, g, b, 0];
            } else {
                controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
            }
        }
        controller.render()?;
        thread::sleep(Duration::from_millis(delay_ms));
        position = (position + 1) % num_leds;
    }
}

fn breathe_animation(controller: &mut rs_ws281x::Controller, num_leds: i32, r: u8, g: u8, b: u8, delay_ms: u64) -> Result<()> {
    println!("Running breathe animation...");
    let mut brightness: f32 = 0.0;
    let mut increasing = true;
    
    loop {
        let factor = brightness / 255.0;
        let current_r = (r as f32 * factor) as u8;
        let current_g = (g as f32 * factor) as u8;
        let current_b = (b as f32 * factor) as u8;
        
        for i in 0..num_leds {
            controller.leds_mut(0)[i as usize] = [current_r, current_g, current_b, 0];
        }
        controller.render()?;
        
        if increasing {
            brightness += 2.0;
            if brightness >= 255.0 {
                brightness = 255.0;
                increasing = false;
            }
        } else {
            brightness -= 2.0;
            if brightness <= 0.0 {
                brightness = 0.0;
                increasing = true;
            }
        }
        
        thread::sleep(Duration::from_millis(delay_ms));
    }
}

fn turn_off(controller: &mut rs_ws281x::Controller, num_leds: i32) -> Result<()> {
    println!("Turning off all LEDs");
    
    for i in 0..num_leds {
        controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
    }
    controller.render()?;
    
    Ok(())
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    
    ((255.0 * (r + m)) as u8, (255.0 * (g + m)) as u8, (255.0 * (b + m)) as u8)
}