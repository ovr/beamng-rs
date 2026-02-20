//! Simple example: connect to BeamNG.tech, pause, step 60 frames, resume.
//!
//! Usage:
//!   cargo run --example simple
//!
//! Requires a running BeamNG.tech instance with the default TCP port (25252).

use beamng_rs::BeamNg;

#[tokio::main]
async fn main() -> beamng_proto::Result<()> {
    // Initialize tracing for debug output.
    tracing_subscriber::fmt::init();

    // Connect to the simulator.
    let bng = BeamNg::new("172.29.160.1", 5555).connect().await?;
    println!("Connected to BeamNG.tech!");

    // Pause the simulation.
    bng.control().pause().await?;
    println!("Simulation paused.");

    // Advance 60 steps (waiting for completion).
    bng.control().step(60, true).await?;
    println!("Stepped 60 frames.");

    // Get the current game state.
    let state = bng.control().get_gamestate().await?;
    println!("Game state: {:?}", state);

    // Resume the simulation.
    bng.control().resume().await?;
    println!("Simulation resumed.");

    // Get system info.
    let info = bng.system().get_info(true, false, false, false).await?;
    println!("System info: {:?}", info);

    // Get gravity.
    let gravity = bng.environment().get_gravity().await?;
    println!("Current gravity: {gravity}");

    // Display a message in the simulator UI.
    bng.ui().display_message("Hello from Rust!").await?;
    println!("Displayed message in simulator.");

    println!("Done!");
    Ok(())
}
