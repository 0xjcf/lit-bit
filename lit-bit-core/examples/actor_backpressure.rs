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
    /// Prevents heap allocation in `no_std` environments by panicking on allocation attempts.
    ///
    /// # Safety
    ///
    /// This function always panics and never returns a valid pointer. It should never be called.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use core::alloc::{GlobalAlloc, Layout};
    ///
    /// let alloc = DummyAlloc;
    /// // This will panic:
    /// unsafe { alloc.alloc(Layout::from_size_align(8, 8).unwrap()); }
    /// ```
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        // Panic immediately to prevent undefined behavior from null pointer dereference
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    /// Deallocates memory, but performs no action as this is a dummy allocator.
///
/// # Safety
///
/// This function is a no-op and does not actually free memory. Using this allocator in a real allocation context will result in undefined behavior.
///
/// # Examples
///
/// ```
/// // No actual deallocation occurs; for demonstration only.
/// unsafe { DUMMY_ALLOC.dealloc(ptr, layout); }
/// ```
unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

// Panic handler for no_std builds
#[cfg(not(feature = "std"))]
use panic_halt as _;

use lit_bit_core::actor::{Actor, ActorError};

#[cfg(feature = "std")]
use std::time::Duration;

/// A simple counter actor that processes messages with configurable delay
#[derive(Debug)]
pub struct CounterActor {
    count: u32,
    processed_messages: u32,
    processing_delay_ms: u32,
}

impl CounterActor {
    #[must_use]
    /// Creates a new `CounterActor` with the specified processing delay in milliseconds.
    ///
    /// The counter and processed message count are initialized to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// let actor = CounterActor::new(100);
    /// assert_eq!(actor.count, 0);
    /// assert_eq!(actor.processed_messages, 0);
    /// assert_eq!(actor.processing_delay_ms, 100);
    /// ```
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

    /// Initializes the counter actor before it begins processing messages.
    ///
    /// Returns `Ok(())` to indicate successful startup.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut actor = CounterActor::new(0);
    /// assert!(actor.on_start().is_ok());
    /// ```
    fn on_start(&mut self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!(
            "🔢 Counter actor starting (delay: {}ms)",
            self.processing_delay_ms
        );
        Ok(())
    }

    #[cfg(feature = "async")]
    /// Processes a `CounterMessage`, updating the counter state and optionally responding to queries.
    ///
    /// Handles increment, decrement, add, and reset operations, as well as asynchronous queries for the current count and statistics when running with the `std` feature. Simulates a configurable processing delay if enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use lit_bit_core::actor::Actor;
    /// use lit_bit_core::examples::actor_backpressure::{CounterActor, CounterMessage};
    ///
    /// let mut actor = CounterActor::new(0);
    /// futures::executor::block_on(actor.on_event(CounterMessage::Increment));
    /// assert_eq!(actor.count, 1);
    /// ```
    fn on_event(&mut self, msg: CounterMessage) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            // Simulate processing delay
            #[cfg(feature = "std")]
            if self.processing_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(u64::from(self.processing_delay_ms)))
                    .await;
            }

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
        })
    }

    #[cfg(not(feature = "async"))]
    /// Processes a `CounterMessage`, updating the counter state and optionally responding to queries.
    ///
    /// Handles increment, decrement, add, and reset operations on the counter, as well as asynchronous queries for the current count and statistics when running with the `std` feature. Simulates a configurable processing delay if enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use lit_bit_core::actor::Actor;
    /// use crate::{CounterActor, CounterMessage};
    ///
    /// let mut actor = CounterActor::new(0);
    /// // Increment the counter
    /// actor.on_event(CounterMessage::Increment).await;
    /// // Add 5 to the counter
    /// actor.on_event(CounterMessage::Add(5)).await;
    /// ```
    fn on_event(&mut self, msg: CounterMessage) -> impl core::future::Future<Output = ()> + Send {
        async move {
            // Simulate processing delay
            #[cfg(feature = "std")]
            if self.processing_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(u64::from(self.processing_delay_ms)))
                    .await;
            }

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
        }
    }
}

#[cfg(feature = "std")]
#[tokio::main]
/// Demonstrates platform-specific back-pressure concepts in the actor system.
///
/// Prints an overview of back-pressure handling strategies for embedded and cloud environments, highlighting differences in feedback and flow control.
///
/// # Returns
/// Returns `Ok(())` if the demonstration completes successfully.
///
/// # Examples
///
/// ```
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     main().await?;
///     Ok(())
/// }
/// ```
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
/// Entry point for embedded (`no_std`) targets.
///
/// This function serves as a placeholder for embedded applications and does not perform any operations.
/// In a real embedded environment, platform-specific initialization or logging would be implemented here.
fn main() {
    // For no_std targets, demonstrate basic concepts
    // In a real embedded application, this would use defmt or similar
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use super::*;

    #[test]
    /// Tests basic operations of the `CounterActor`, including message processing and state updates.
    ///
    /// This test verifies that the actor correctly handles `Add`, `Increment`, and `Decrement` messages,
    /// and that its internal state reflects the expected count and processed message count after processing.
    ///
    /// # Examples
    ///
    /// ```
    /// counter_actor_basic_operations();
    /// ```
    fn counter_actor_basic_operations() {
        let mut counter = CounterActor::new(0);

        // Test lifecycle
        assert!(counter.on_start().is_ok());

        // Test message processing
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            counter.on_event(CounterMessage::Add(5)).await;
            counter.on_event(CounterMessage::Increment).await;
            counter.on_event(CounterMessage::Decrement).await;

            assert_eq!(counter.count, 5); // 0 + 5 + 1 - 1 = 5
            assert_eq!(counter.processed_messages, 3);
        });
    }
}
