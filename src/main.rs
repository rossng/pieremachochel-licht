use anyhow::Result;
use clap::{Parser, ValueEnum};
use rand::Rng;
use rs_ws281x::{ChannelBuilder, ControllerBuilder, StripType};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

#[derive(Parser)]
#[command(name = "pm-licht")]
#[command(about = "LED light controller with multiple animation modes")]
struct Cli {
    #[arg(short, long, default_value_t = 8)]
    num_leds: i32,

    #[arg(short, long, default_value_t = 18)]
    gpio_pin: i32,

    #[arg(short, long, default_value_t = 255)]
    brightness: u8,

    #[arg(short, long, default_value_t = 250)]
    delay_ms: u64,

    #[arg(short, long, value_enum, default_value = "flash")]
    mode: Mode,

    #[arg(short, long, default_value_t = false)]
    flipped: bool,

    #[arg(long, default_value_t = 30)]
    mode_duration_secs: u64,

    #[arg(long, default_value_t = false)]
    big_leds: bool,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Mode {
    Chase,
    Flash,
    MultiChase,
    Alternate,
    Bounce,
    FillEmpty,
    Juggle,
    Theater,
}

impl Mode {
    fn random_different_from(&self) -> Self {
        let mut rng = rand::thread_rng();
        let modes = [Mode::Chase, Mode::Flash, Mode::MultiChase, Mode::Alternate, Mode::Bounce, Mode::FillEmpty, Mode::Juggle, Mode::Theater];
        let available: Vec<_> = modes.iter().filter(|&&m| m != *self).copied().collect();
        available[rng.gen_range(0..available.len())]
    }
    
    fn name(&self) -> &str {
        match self {
            Mode::Chase => "Chase",
            Mode::Flash => "Flash",
            Mode::MultiChase => "MultiChase",
            Mode::Alternate => "Alternate",
            Mode::Bounce => "Bounce",
            Mode::FillEmpty => "FillEmpty",
            Mode::Juggle => "Juggle",
            Mode::Theater => "Theater",
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct IpcCommand {
    command: Vec<serde_json::Value>,
}

#[derive(Clone)]
struct AppState {
    speed: f64,
}

impl AppState {
    fn new() -> Self {
        Self { speed: 1.0 }
    }
    
    fn get_delay_ms(&self, base_delay_ms: u64) -> u64 {
        (base_delay_ms as f64 / self.speed) as u64
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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

    let app_state = Arc::new(Mutex::new(AppState::new()));
    
    let socket_path = "/tmp/pm-licht";
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }
    
    let listener = UnixListener::bind(socket_path)?;
    let state_clone = Arc::clone(&app_state);
    
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let state = Arc::clone(&state_clone);
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, state).await {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    });

    run_animation(&mut controller, cli.num_leds, cli.delay_ms, cli.mode, cli.flipped, cli.mode_duration_secs, cli.big_leds, app_state)?;

    Ok(())
}

async fn handle_client(stream: UnixStream, state: Arc<Mutex<AppState>>) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    
    while reader.read_line(&mut line).await? > 0 {
        if let Ok(cmd) = serde_json::from_str::<IpcCommand>(&line.trim()) {
            if cmd.command.len() >= 3 
                && cmd.command[0].as_str() == Some("set_property") 
                && cmd.command[1].as_str() == Some("speed") {
                if let Some(speed_value) = cmd.command[2].as_f64() {
                    let mut app_state = state.lock().unwrap();
                    app_state.speed = speed_value;
                    println!("Speed set to: {}", speed_value);
                }
            }
        }
        line.clear();
    }
    
    Ok(())
}

fn flip_leds(leds: &mut [[u8; 4]], num_leds: i32) {
    let mut temp = vec![[0u8; 4]; num_leds as usize];
    for i in 0..num_leds {
        temp[(num_leds - 1 - i) as usize] = leds[i as usize];
    }
    for i in 0..num_leds {
        leds[i as usize] = temp[i as usize];
    }
}

fn run_animation(controller: &mut rs_ws281x::Controller, num_leds: i32, base_delay_ms: u64, initial_mode: Mode, initial_flipped: bool, mode_duration_secs: u64, big_leds: bool, app_state: Arc<Mutex<AppState>>) -> Result<()> {
    println!("Starting LED animation with {} mode{}", initial_mode.name(), if initial_flipped { " (flipped)" } else { "" });
    
    let warm_white = if big_leds {
        // Big LEDs (B, R, G, W)
        [25, 255, 160, 0]
    } else {
        // Small LEDs (B, G, R, W) - cozy orange-tinted white for RGB LEDs
        [30, 170, 255, 0]
    };
    
    let mut current_mode = initial_mode;
    let mut is_flipped = initial_flipped;
    let mode_duration = Duration::from_secs(mode_duration_secs);
    let mut mode_start = Instant::now();
    
    let mut chase_position = 0;
    let mut flash_state = false;
    let mut alternate_state = false;
    let mut bounce_position = 0;
    let mut bounce_direction = 1;
    let mut fill_position = 0;
    let mut fill_is_filling = true;
    let mut theater_offset = 0;
    let mut juggle_positions = [0.0f32, 0.0f32, 0.0f32];
    let mut juggle_velocities = [0.3f32, 0.5f32, 0.7f32];
    
    loop {
        // Check if it's time to switch modes
        if mode_start.elapsed() >= mode_duration {
            current_mode = current_mode.random_different_from();
            // Randomly decide whether to flip the new mode
            let mut rng = rand::thread_rng();
            is_flipped = rng.gen_bool(0.5);
            mode_start = Instant::now();
            println!("Switching to {} mode{}", current_mode.name(), if is_flipped { " (flipped)" } else { "" });
        }
        
        // Run the appropriate mode
        match current_mode {
            Mode::Chase => {
                run_chase_step(controller, num_leds, &mut chase_position, warm_white)?;
            },
            Mode::Flash => {
                run_flash_step(controller, num_leds, &mut flash_state, warm_white)?;
            },
            Mode::MultiChase => {
                run_multi_chase_step(controller, num_leds, &mut chase_position, warm_white)?;
            },
            Mode::Alternate => {
                run_alternate_step(controller, num_leds, &mut alternate_state, warm_white)?;
            },
            Mode::Bounce => {
                run_bounce_step(controller, num_leds, &mut bounce_position, &mut bounce_direction, warm_white)?;
            },
            Mode::FillEmpty => {
                run_fill_empty_step(controller, num_leds, &mut fill_position, &mut fill_is_filling, warm_white)?;
            },
            Mode::Theater => {
                run_theater_step(controller, num_leds, &mut theater_offset, warm_white)?;
            },
            Mode::Juggle => {
                run_juggle_step(controller, num_leds, &mut juggle_positions, &mut juggle_velocities, warm_white)?;
            },
        }
        
        // Apply flipping if enabled
        if is_flipped {
            flip_leds(controller.leds_mut(0), num_leds);
        }
        
        controller.render()?;
        
        let current_delay = {
            let state = app_state.lock().unwrap();
            state.get_delay_ms(base_delay_ms)
        };
        
        thread::sleep(Duration::from_millis(current_delay));
    }
}

fn run_chase_step(controller: &mut rs_ws281x::Controller, num_leds: i32, position: &mut i32, color: [u8; 4]) -> Result<()> {
    for i in 0..num_leds {
        if i == *position {
            controller.leds_mut(0)[i as usize] = color;
        } else {
            controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
        }
    }
    *position = (*position + 1) % num_leds;
    Ok(())
}

fn run_flash_step(controller: &mut rs_ws281x::Controller, num_leds: i32, state: &mut bool, color: [u8; 4]) -> Result<()> {
    let led_color = if *state { color } else { [0, 0, 0, 0] };
    
    for i in 0..num_leds {
        controller.leds_mut(0)[i as usize] = led_color;
    }
    
    *state = !*state;
    Ok(())
}

fn run_multi_chase_step(controller: &mut rs_ws281x::Controller, num_leds: i32, position: &mut i32, color: [u8; 4]) -> Result<()> {
    for i in 0..num_leds {
        let is_lit = (0..3).any(|offset| {
            let pos = (*position + offset) % num_leds;
            i == pos
        });
        
        if is_lit {
            controller.leds_mut(0)[i as usize] = color;
        } else {
            controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
        }
    }
    *position = (*position + 1) % num_leds;
    Ok(())
}

fn run_alternate_step(controller: &mut rs_ws281x::Controller, num_leds: i32, state: &mut bool, color: [u8; 4]) -> Result<()> {
    for i in 0..num_leds {
        let is_lit = if *state {
            i % 2 == 0
        } else {
            i % 2 == 1
        };
        
        if is_lit {
            controller.leds_mut(0)[i as usize] = color;
        } else {
            controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
        }
    }
    
    *state = !*state;
    Ok(())
}

fn run_bounce_step(controller: &mut rs_ws281x::Controller, num_leds: i32, position: &mut i32, direction: &mut i32, color: [u8; 4]) -> Result<()> {
    // For 8 LEDs: positions 0,1,2,3 | 4,5,6,7
    // We want to light pairs at distance from center:
    // position 0: LEDs 3,4 (center pair)
    // position 1: LEDs 2,5
    // position 2: LEDs 1,6  
    // position 3: LEDs 0,7 (outer pair)
    
    let center_left = (num_leds - 1) / 2;
    let center_right = num_leds / 2;
    
    for i in 0..num_leds {
        // Calculate which LEDs should be lit based on position
        let left_led = center_left - *position;
        let right_led = center_right + *position;
        
        let is_lit = i == left_led || i == right_led;
        
        if is_lit && left_led >= 0 && right_led < num_leds {
            controller.leds_mut(0)[i as usize] = color;
        } else {
            controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
        }
    }
    
    *position += *direction;
    
    // Bounce when we reach the edges
    let max_position = num_leds / 2;
    if *position >= max_position || *position < 0 {
        *direction = -*direction;
        *position += *direction;
    }
    
    Ok(())
}

fn run_fill_empty_step(controller: &mut rs_ws281x::Controller, num_leds: i32, position: &mut i32, is_filling: &mut bool, color: [u8; 4]) -> Result<()> {
    for i in 0..num_leds {
        if *is_filling {
            // When filling, light up all LEDs from 0 to position (inclusive)
            if i <= *position {
                controller.leds_mut(0)[i as usize] = color;
            } else {
                controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
            }
        } else {
            // When emptying, turn off LEDs from 0 to position (inclusive)
            if i <= *position {
                controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
            } else {
                controller.leds_mut(0)[i as usize] = color;
            }
        }
    }
    
    *position += 1;
    
    // When we reach the end, switch between filling and emptying
    if *position >= num_leds {
        *is_filling = !*is_filling;
        *position = 0;
    }
    
    Ok(())
}

fn run_theater_step(controller: &mut rs_ws281x::Controller, num_leds: i32, offset: &mut i32, color: [u8; 4]) -> Result<()> {
    for i in 0..num_leds {
        // Light only one pair at a time: (0,1), then (2,3), then (4,5), etc
        let current_pair = *offset;
        let pair_start = current_pair * 2;
        let is_lit = i == pair_start || i == pair_start + 1;
        
        if is_lit {
            controller.leds_mut(0)[i as usize] = color;
        } else {
            controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
        }
    }
    
    *offset = (*offset + 1) % (num_leds / 2);
    Ok(())
}

fn run_juggle_step(controller: &mut rs_ws281x::Controller, num_leds: i32, positions: &mut [f32; 3], velocities: &mut [f32; 3], color: [u8; 4]) -> Result<()> {
    // Clear all LEDs first
    for i in 0..num_leds {
        controller.leds_mut(0)[i as usize] = [0, 0, 0, 0];
    }
    
    // Update and render each juggling ball
    for i in 0..3 {
        // Update position
        positions[i] += velocities[i];
        
        // Bounce off walls
        if positions[i] >= (num_leds - 1) as f32 || positions[i] <= 0.0 {
            velocities[i] = -velocities[i];
            positions[i] = positions[i].clamp(0.0, (num_leds - 1) as f32);
        }
        
        // Light up the LED at this position
        let led_index = positions[i].round() as usize;
        if led_index < num_leds as usize {
            controller.leds_mut(0)[led_index] = color;
        }
    }
    
    Ok(())
}