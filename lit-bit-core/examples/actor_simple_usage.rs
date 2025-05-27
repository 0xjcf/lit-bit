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
    /// Panics on any heap allocation attempt in a `no_std` context.
    ///
    /// This allocator is intended as a placeholder to prevent heap usage in environments
    /// where dynamic memory allocation is not supported. Any call to `alloc` will immediately panic.
    ///
    /// # Safety
    ///
    /// This function always panics and never returns a valid pointer.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use your_crate::DummyAlloc;
    /// use core::alloc::Layout;
    ///
    /// let alloc = DummyAlloc;
    /// // This will panic
    /// unsafe { alloc.alloc(Layout::from_size_align(8, 8).unwrap()); }
    /// ```
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        // Panic immediately to prevent undefined behavior from null pointer dereference
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    /// Used in environments where dynamic memory allocation is not supported.
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
    /// Creates a new `SimpleActor` instance with the count initialized to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// let actor = SimpleActor::new();
    /// assert_eq!(actor.count, 0);
    /// ```
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for SimpleActor {
    type Message = u32;

    #[cfg(feature = "async")]
    /// Handles an incoming message by incrementing the actor's count asynchronously.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut actor = SimpleActor::new();
    /// futures::executor::block_on(actor.on_event(5));
    /// assert_eq!(actor.count, 5);
    /// ```
    fn on_event(&mut self, msg: u32) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            self.count += msg;
            #[cfg(feature = "std")]
            println!("Count is now: {}", self.count);
        })
    }

    #[cfg(not(feature = "async"))]
    /// Handles an incoming message by incrementing the actor's count asynchronously.
    ///
    /// Increments the internal `count` by the value of the received message. If the `std` feature is enabled, prints the updated count to the console.
    ///
    /// # Parameters
    /// - `msg`: The value to add to the actor's count.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut actor = SimpleActor::new();
    /// futures::executor::block_on(actor.on_event(5));
    /// assert_eq!(actor.count, 5);
    /// ```
    fn on_event(&mut self, msg: u32) -> impl core::future::Future<Output = ()> + Send {
        async move {
            self.count += msg;
            #[cfg(feature = "std")]
            println!("Count is now: {}", self.count);
        }
    }

    /// Invoked when the actor starts, allowing for initialization logic.
    ///
    /// Returns `Ok(())` to indicate successful startup.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut actor = SimpleActor::new();
    /// assert!(actor.on_start().is_ok());
    /// ```
    fn on_start(&mut self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!("SimpleActor starting with count: {}", self.count);
        Ok(())
    }

    /// Handles cleanup logic when the actor stops.
    ///
    /// Returns `Ok(())` after performing any necessary shutdown actions.
    ///
    /// # Examples
    ///
    /// ```
    /// let actor = SimpleActor::new();
    /// let result = actor.on_stop();
    /// assert!(result.is_ok());
    /// ```
    fn on_stop(self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!("SimpleActor stopping with final count: {}", self.count);
        Ok(())
    }

    /// Specifies that the actor should be restarted individually if a panic occurs.
    ///
    /// Always returns `RestartStrategy::OneForOne`, indicating only this actor will be restarted on panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lit_bit_core::{Actor, RestartStrategy};
    /// # struct SimpleActor;
    /// # impl Actor for SimpleActor {
    /// #     type Message = u32;
    /// #     fn on_event(&mut self, _msg: u32) -> core::result::Result<(), ()> { Ok(()) }
    /// #     fn on_start(&mut self) -> core::result::Result<(), ()> { Ok(()) }
    /// #     fn on_stop(&mut self) -> core::result::Result<(), ()> { Ok(()) }
    ///     fn on_panic(&self, _info: &core::panic::PanicInfo) -> RestartStrategy {
    ///         RestartStrategy::OneForOne
    ///     }
    /// # }
    /// let actor = SimpleActor;
    /// assert_eq!(actor.on_panic(&core::panic::PanicInfo::internal_constructor(None, &[])), RestartStrategy::OneForOne);
    /// ```
    fn on_panic(&self, _info: &core::panic::PanicInfo) -> RestartStrategy {
        RestartStrategy::OneForOne
    }
}

#[cfg(feature = "std")]
#[tokio::main]
/// Runs an example demonstrating the usage of re-exported actor types from the `lit_bit_core` crate root.
///
/// Creates a `SimpleActor`, sends messages to it, and prints the final count to showcase direct imports of actor-related types.
///
/// # Returns
/// Returns `Ok(())` if the example completes successfully, or an error if one occurs.
///
/// # Examples
///
/// ```no_run
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // This will print example output and demonstrate actor usage.
///     main().await
/// }
/// ```
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
/// Entry point for `no_std` environments to verify accessibility of re-exported actor types.
///
/// Instantiates `SimpleActor` and references `ActorError`, `RestartStrategy`, and `SendError` to ensure they are available in `no_std` builds. No runtime logic is performed.
fn main() {
    // For no_std targets, just demonstrate that the types are accessible
    let _actor = SimpleActor::new();
    let _error = ActorError::StartupFailure;
    let _strategy = RestartStrategy::OneForOne;
    let _send_error = SendError::Full(42u32);

    // This compiles successfully, proving the re-exports work for no_std too
}
