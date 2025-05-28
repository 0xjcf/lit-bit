//! Embassy Actor Simple Example
//!
//! This example demonstrates how to use the Embassy async actor system with static allocation.
//! It shows the basic pattern for creating Embassy actors with zero-heap operation.
//!
//! ## Usage
//!
//! This is a library function that can be integrated into your Embassy application:
//!
//! ```rust,no_run
//! use embassy_executor::Spawner;
//! use lit_bit_core::examples::embassy_actor_simple::run_embassy_actor_example;
//!
//! #[embassy_executor::main]
//! async fn main(spawner: Spawner) {
//!     run_embassy_actor_example(spawner).await;
//! }
//! ```
//!
//! ## Features Required
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! lit-bit-core = { version = "0.0.1-alpha.0", features = ["async-embassy"] }
//! embassy-executor = { version = "0.6", features = ["arch-cortex-m", "executor-thread", "integrated-timers"] }
//! embassy-sync = "0.6"
//! static-cell = "2.0"
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

// Panic handler for no_std builds
#[cfg(all(not(feature = "std"), feature = "panic-halt"))]
use panic_halt as _;

// Alternative panic handler when panic-halt is not available
#[cfg(all(not(feature = "std"), not(feature = "panic-halt")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use embassy_executor::Spawner;
use lit_bit_core::{
    actor::spawn::{CounterActor, spawn_counter_actor_embassy},
    static_embassy_channel,
};

/// Runs the Embassy actor example.
///
/// This function demonstrates:
/// - Creating a static Embassy channel for actor communication
/// - Spawning a concrete actor (`CounterActor`) on the Embassy executor
/// - Sending messages to the actor using the Address
///
/// ## Memory Usage
///
/// - Zero heap allocation
/// - Static channel allocation using `StaticCell`
/// - Predictable memory usage at compile time
///
/// ## Integration
///
/// This function is designed to be called from your Embassy main function:
///
/// ```rust,no_run
/// #[embassy_executor::main]
/// async fn main(spawner: Spawner) {
///     run_embassy_actor_example(spawner).await;
/// }
/// ```
///
/// # Panics
///
/// Panics if the Embassy task arena is full and the actor cannot be spawned.
pub async fn run_embassy_actor_example(spawner: Spawner) {
    // Create a static channel for the actor
    // This uses StaticCell for zero-heap allocation
    let (sender, receiver) = static_embassy_channel!(COUNTER_CHANNEL: u32, 16);

    // Create the actor instance
    let actor = CounterActor::new();

    // Spawn the actor on the Embassy executor
    let address = spawn_counter_actor_embassy(spawner, actor, sender, receiver)
        .expect("Failed to spawn actor");

    // Send some messages to the actor
    // Embassy channels are infallible, so we can use expect() safely
    address
        .send(10)
        .await
        .expect("Embassy send should never fail");
    address
        .send(20)
        .await
        .expect("Embassy send should never fail");

    // Alternative: handle the Result explicitly (though it's always Ok in Embassy)
    match address.send(30).await {
        Ok(()) => {
            // This is the normal path for Embassy
        }
        Err(_) => {
            // This should never happen in Embassy 0.6, but provides API consistency
            panic!("Unexpected send failure in Embassy");
        }
    }

    // In a real application, you would continue with your main logic here
    // The actor will continue running and processing messages
}

/// Example main function showing how to integrate the Embassy actor example.
///
/// In a real Embassy application, you would call `run_embassy_actor_example` from your main function.
/// This example shows the pattern but cannot actually run without proper Embassy executor setup.
#[cfg(feature = "std")]
fn main() {
    println!("Embassy Actor Example");
    println!("====================");
    println!();
    println!("This is a library function for Embassy applications.");
    println!("To use it, add this to your Embassy main function:");
    println!();
    println!("```rust");
    println!("#[embassy_executor::main]");
    println!("async fn main(spawner: Spawner) {{");
    println!("    run_embassy_actor_example(spawner).await;");
    println!("}}");
    println!("```");
    println!();
    println!("Required features: async-embassy");
    println!("Required dependencies: embassy-executor, embassy-sync, static-cell");
}

#[cfg(not(feature = "std"))]
fn main() {
    // For no_std builds, we can't use println! so we just provide a stub
    // In a real embedded application, this would be replaced with your target-specific main
}
