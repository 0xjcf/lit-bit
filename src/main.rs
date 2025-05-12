// Basic Agent Entry Point
use anyhow::Result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Define modules (consider creating these files/dirs)
mod agent; // Declare the agent module
// mod config;
// mod errors;

#[tokio::main]
async fn main() -> Result<()> {
    // Basic tracing setup (reads RUST_LOG env var)
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    tracing::info!("Creating agent...");

    // Placeholder: Load config
    // let config = config::load()?;

    // Create agent instance
    let agent = agent::Agent::new(); // Use the new() method

    // Start the main agent loop
    tracing::info!("Starting agent run loop...");
    agent.run().await // run() returns Result, handle or propagate it

    // Note: The program will likely only exit here if agent.run() returns
    // or due to external signal (Ctrl+C), as the loop in run() is infinite.
    // tracing::info!("Agent finished.");
    // Ok(()) // run() already returns Result
} 