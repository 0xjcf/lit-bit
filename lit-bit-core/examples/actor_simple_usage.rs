//! Simple example demonstrating the use of re-exported actor types from the top level.
//! This example shows how users can now import actor types directly from `lit_bit_core`
//! instead of having to use longer paths like `lit_bit_core::actor::Actor`.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

// Import actor types directly from the crate root - this is what the re-exports enable!
use lit_bit_core::{Actor, ActorError, RestartStrategy};

#[cfg(not(feature = "std"))]
use lit_bit_core::SendError;

// Required for no_std builds
#[cfg(not(feature = "std"))]
extern crate alloc;

// Dummy allocator for no_std builds
#[cfg(not(feature = "std"))]
#[global_allocator]
static DUMMY: DummyAlloc = DummyAlloc;

#[cfg(not(feature = "std"))]
struct DummyAlloc;

#[cfg(not(feature = "std"))]
unsafe impl core::alloc::GlobalAlloc for DummyAlloc {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        // Panic immediately to prevent undefined behavior from null pointer dereference
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

// Panic handler for no_std builds
#[cfg(not(feature = "std"))]
use panic_halt as _;

/// A simple counter actor to demonstrate the re-exported types
#[derive(Debug)]
struct SimpleActor {
    count: u32,
}

impl SimpleActor {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for SimpleActor {
    type Message = u32;

    async fn on_event(&mut self, msg: u32) {
        self.count += msg;
        #[cfg(feature = "std")]
        println!("Count is now: {}", self.count);
    }

    fn on_start(&mut self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!("SimpleActor starting with count: {}", self.count);
        Ok(())
    }

    fn on_stop(self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!("SimpleActor stopping with final count: {}", self.count);
        Ok(())
    }

    fn on_panic(&self, _info: &core::panic::PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }
}

#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Simple Actor Usage Example");
    println!("==============================");
    println!("This example demonstrates using re-exported actor types from lit_bit_core root.");

    // Create and start the actor
    let mut actor = SimpleActor::new();
    println!("Actor created with re-exported types!");

    // Test the actor
    actor.on_event(5).await;
    actor.on_event(10).await;

    println!("Final count: {}", actor.count);
    println!("\n✅ Example completed successfully!");
    println!("   All actor types were imported directly from lit_bit_core root!");

    Ok(())
}

#[cfg(not(feature = "std"))]
fn main() {
    // For no_std targets, just demonstrate that the types are accessible
    let _actor = SimpleActor::new();
    let _error = ActorError::StartupFailure;
    let _strategy = RestartStrategy::OneForOne;
    let _send_error = SendError::Full(42u32);

    // This compiles successfully, proving the re-exports work for no_std too
}
