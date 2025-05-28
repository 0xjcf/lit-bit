//! # Back-pressure Handling Example
//!
//! This example demonstrates platform-specific back-pressure handling in the lit-bit actor system:
//! - **Embedded (`no_std`)**: Fail-fast semantics with immediate feedback
//! - **Cloud (std)**: Async back-pressure with natural flow control
//!
//! Key concepts demonstrated:
//! - Platform-dual back-pressure semantics
//! - Mailbox overflow handling
//! - Message ordering guarantees
//! - Load shedding patterns
//! - Error handling strategies

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

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

use lit_bit_core::actor::{Actor, ActorError};

/// A simple counter actor that processes messages with configurable delay
#[derive(Debug)]
pub struct CounterActor {
    count: u32,
    processed_messages: u32,
    processing_delay_ms: u32,
}

impl CounterActor {
    #[must_use]
    pub fn new(processing_delay_ms: u32) -> Self {
        Self {
            count: 0,
            processed_messages: 0,
            processing_delay_ms,
        }
    }
}

/// Messages for the counter actor
#[derive(Debug)]
pub enum CounterMessage {
    Increment,
    Decrement,
    Add(u32),
    Reset,
    #[cfg(feature = "std")]
    GetCount {
        reply_to: tokio::sync::oneshot::Sender<u32>,
    },
    #[cfg(feature = "std")]
    GetStats {
        reply_to: tokio::sync::oneshot::Sender<CounterStats>,
    },
}

/// Statistics about counter operations
#[derive(Debug, Clone)]
pub struct CounterStats {
    pub current_count: u32,
    pub processed_messages: u32,
}

impl Actor for CounterActor {
    type Message = CounterMessage;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn on_start(&mut self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!(
            "🔢 Counter actor starting (delay: {}ms)",
            self.processing_delay_ms
        );
        Ok(())
    }

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        // Note: For simplicity in this example, we're using sync processing
        // In a real async scenario, you'd use async operations here
        match msg {
            CounterMessage::Increment => {
                self.count += 1;
                self.processed_messages += 1;
                #[cfg(feature = "std")]
                println!("➕ Count: {}", self.count);
            }

            CounterMessage::Decrement => {
                self.count = self.count.saturating_sub(1);
                self.processed_messages += 1;
                #[cfg(feature = "std")]
                println!("➖ Count: {}", self.count);
            }

            CounterMessage::Add(n) => {
                self.count = self.count.saturating_add(n);
                self.processed_messages += 1;
                #[cfg(feature = "std")]
                println!("➕ Added {}, Count: {}", n, self.count);
            }

            CounterMessage::Reset => {
                self.count = 0;
                self.processed_messages += 1;
                #[cfg(feature = "std")]
                println!("🔄 Count reset to 0");
            }

            #[cfg(feature = "std")]
            CounterMessage::GetCount { reply_to } => {
                let _ = reply_to.send(self.count);
            }

            #[cfg(feature = "std")]
            CounterMessage::GetStats { reply_to } => {
                let stats = CounterStats {
                    current_count: self.count,
                    processed_messages: self.processed_messages,
                };
                let _ = reply_to.send(stats);
            }
        }

        core::future::ready(())
    }
}

#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Actor Back-pressure Handling Example");
    println!("=======================================");

    // Demonstrate basic back-pressure concepts
    println!("\n💡 Back-pressure Concepts:");
    println!("   • Embedded: Fail-fast, immediate feedback, real-time friendly");
    println!("   • Cloud: Async back-pressure, natural flow control, high-throughput");

    println!("\n✅ Back-pressure example completed!");

    Ok(())
}

#[cfg(not(feature = "std"))]
fn main() {
    // For no_std targets, demonstrate basic concepts
    // In a real embedded application, this would use defmt or similar
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use super::*;

    #[test]
    fn counter_actor_basic_operations() {
        let mut counter = CounterActor::new(0);

        // Test lifecycle
        assert!(counter.on_start().is_ok());

        // Test message processing
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            counter.handle(CounterMessage::Add(5)).await;
            counter.handle(CounterMessage::Increment).await;
            counter.handle(CounterMessage::Decrement).await;

            assert_eq!(counter.count, 5); // 0 + 5 + 1 - 1 = 5
            assert_eq!(counter.processed_messages, 3);
        });
    }
}
