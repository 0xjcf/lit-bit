//! # Mailbox Patterns in lit-bit
//!
//! This example demonstrates the two primary patterns for creating actor mailboxes in lit-bit:
//!
//! ## Pattern 1: Using `static_mailbox!` Macro (Recommended)
//!
//! The `static_mailbox!` macro provides a simple, ergonomic way to create static mailboxes:
//!
//! ```rust,no_run
//! use lit_bit_core::static_mailbox;
//!
//! // Create a mailbox with capacity 16
//! let (producer, consumer) = static_mailbox!(MY_MAILBOX: MyMessage, 16);
//!
//! // Send messages
//! producer.enqueue(MyMessage::Command).unwrap();
//!
//! // Receive messages
//! while let Some(msg) = consumer.dequeue() {
//!     // Process message...
//! }
//! ```
//!
//! ## Pattern 2: Using `create_mailbox` with `StaticCell` (Advanced)
//!
//! For more control over static allocation, use `create_mailbox` with a `StaticCell`:
//!
//! ```rust,no_run
//! use lit_bit_core::create_mailbox;
//! use static_cell::StaticCell;
//! use heapless::spsc::Queue;
//!
//! // Define static storage with custom attributes
//! #[cfg_attr(not(feature = "std"), link_section = ".fast_memory")]
//! static QUEUE: StaticCell<Queue<MyMessage, 8>> = StaticCell::new();
//!
//! // Create mailbox from static cell
//! let (producer, consumer) = create_mailbox(&QUEUE);
//! ```
//!
//! ## Features
//!
//! - **Zero Allocation**: Both patterns use static memory, no heap allocation
//! - **Memory Placement**: Control where queues are placed in memory
//! - **Backpressure**: Built-in handling for full queues
//! - **Platform Support**: Works in both `std` and `no_std` environments
//!
//! ## Example Implementation
//!
//! This example implements a temperature control system with:
//! - A sensor actor using `static_mailbox!`
//! - An actuator actor using `create_mailbox`
//! - Message handling and error management
//! - Platform-specific optimizations
//!
//! Run with:
//! ```bash
//! # Run in std mode
//! cargo run --example mailbox_patterns
//!
//! # Run in no_std mode
//! cargo run --example mailbox_patterns --no-default-features
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

use heapless::spsc::Queue;
use lit_bit_core::{Actor, create_mailbox, static_mailbox};
use static_cell::StaticCell;

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
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[cfg(not(feature = "std"))]
use panic_halt as _;

/// Messages that can be sent to the sensor actor.
///
/// This demonstrates a typical message enum for an IoT sensor:
/// - Commands to read different values
/// - Configuration commands with parameters
#[derive(Debug)]
enum SensorMessage {
    /// Request a temperature reading
    ReadTemperature,
    /// Request a humidity reading
    ReadHumidity,
    /// Set the temperature threshold for alerts
    SetThreshold(f32),
}

/// Messages that can be sent to the actuator actor.
///
/// This demonstrates a typical message enum for a control device:
/// - Binary state commands (on/off)
/// - Analog control commands (power level)
#[derive(Debug)]
enum ActuatorMessage {
    /// Turn the actuator on
    TurnOn,
    /// Turn the actuator off
    TurnOff,
    /// Set the power level (0-100%)
    SetPower(u8),
}

/// A sensor actor that demonstrates the `static_mailbox!` pattern.
///
/// This actor shows how to:
/// - Use the recommended mailbox pattern
/// - Handle different message types
/// - Maintain actor state
/// - Work in both std and no_std environments
struct SensorActor {
    /// Temperature threshold for alerts
    threshold: f32,
    /// Last temperature reading
    last_reading: f32,
}

impl SensorActor {
    /// Creates a new sensor actor with default settings.
    fn new() -> Self {
        Self {
            threshold: 25.0,
            last_reading: 0.0,
        }
    }

    /// Simulates reading from a temperature sensor.
    fn read_temperature(&mut self) -> f32 {
        // Simulate reading from a sensor
        self.last_reading = 23.5;
        self.last_reading
    }

    /// Simulates reading from a humidity sensor.
    fn read_humidity(&mut self) -> f32 {
        // Simulate reading from a sensor
        75.0
    }
}

impl Actor for SensorActor {
    type Message = SensorMessage;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        match msg {
            SensorMessage::ReadTemperature => {
                let temp = self.read_temperature();
                #[cfg(feature = "std")]
                println!("Temperature: {:.1}°C", temp);
            }
            SensorMessage::ReadHumidity => {
                let humidity = self.read_humidity();
                #[cfg(feature = "std")]
                println!("Humidity: {:.1}%", humidity);
            }
            SensorMessage::SetThreshold(value) => {
                self.threshold = value;
                #[cfg(feature = "std")]
                println!("Threshold set to: {:.1}°C", value);
            }
        }
        core::future::ready(())
    }
}

/// An actuator actor that demonstrates the `create_mailbox` pattern.
///
/// This actor shows how to:
/// - Use the advanced mailbox pattern
/// - Place mailboxes in specific memory sections
/// - Handle backpressure from full queues
struct ActuatorActor {
    /// Current power level (0-100%)
    power: u8,
    /// Whether the actuator is on
    is_on: bool,
}

impl ActuatorActor {
    /// Creates a new actuator actor in the off state.
    fn new() -> Self {
        Self {
            power: 0,
            is_on: false,
        }
    }
}

impl Actor for ActuatorActor {
    type Message = ActuatorMessage;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        match msg {
            ActuatorMessage::TurnOn => {
                self.is_on = true;
                #[cfg(feature = "std")]
                println!("Actuator turned ON at power: {}%", self.power);
            }
            ActuatorMessage::TurnOff => {
                self.is_on = false;
                #[cfg(feature = "std")]
                println!("Actuator turned OFF");
            }
            ActuatorMessage::SetPower(value) => {
                self.power = value;
                #[cfg(feature = "std")]
                println!("Actuator power set to: {}%", value);
            }
        }
        core::future::ready(())
    }
}

// Static cell for the actuator mailbox (demonstrates manual control)
// Place in fast memory section for embedded targets
#[cfg_attr(not(feature = "std"), link_section = ".fast_memory")]
static ACTUATOR_QUEUE: StaticCell<Queue<ActuatorMessage, 8>> = StaticCell::new();

/// Example showing mailbox patterns in a std environment.
#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Mailbox Patterns Example");
    println!("===========================");

    // Pattern 1: Using static_mailbox! macro (recommended)
    println!("\n📫 Pattern 1: static_mailbox! macro");
    let (mut sensor_tx, mut sensor_rx) = static_mailbox!(SENSOR_MAILBOX: SensorMessage, 16);
    let mut sensor = SensorActor::new();

    // Send some messages
    sensor_tx
        .enqueue(SensorMessage::SetThreshold(30.0))
        .unwrap();
    sensor_tx.enqueue(SensorMessage::ReadTemperature).unwrap();
    sensor_tx.enqueue(SensorMessage::ReadHumidity).unwrap();

    // Process messages
    while let Some(msg) = sensor_rx.dequeue() {
        sensor.handle(msg).await;
    }

    // Pattern 2: Using create_mailbox with StaticCell
    println!("\n📬 Pattern 2: create_mailbox with StaticCell");
    let (mut actuator_tx, mut actuator_rx) = create_mailbox(&ACTUATOR_QUEUE);
    let mut actuator = ActuatorActor::new();

    // Send some messages
    actuator_tx.enqueue(ActuatorMessage::SetPower(75)).unwrap();
    actuator_tx.enqueue(ActuatorMessage::TurnOn).unwrap();

    // Demonstrate backpressure when queue is full
    for i in 0..10 {
        match actuator_tx.enqueue(ActuatorMessage::SetPower(i * 10)) {
            Ok(_) => println!("Message enqueued successfully"),
            Err(_) => println!("Queue full, message dropped"),
        }
    }

    // Process messages
    while let Some(msg) = actuator_rx.dequeue() {
        actuator.handle(msg).await;
    }

    actuator_tx.enqueue(ActuatorMessage::TurnOff).unwrap();
    if let Some(msg) = actuator_rx.dequeue() {
        actuator.handle(msg).await;
    }

    println!("\n✅ Example completed successfully!");
    Ok(())
}

/// Example showing mailbox patterns in a no_std environment.
#[cfg(not(feature = "std"))]
#[no_mangle]
fn main() -> ! {
    // Create actors and mailboxes
    let (sensor_tx, mut sensor_rx) = static_mailbox!(SENSOR_MAILBOX: SensorMessage, 16);
    let mut sensor = SensorActor::new();

    let (actuator_tx, mut actuator_rx) = create_mailbox(&ACTUATOR_QUEUE);
    let mut actuator = ActuatorActor::new();

    // Main control loop
    loop {
        // Process sensor messages
        if let Some(msg) = sensor_rx.dequeue() {
            let _ = sensor.handle(msg);
        }

        // Process actuator messages
        if let Some(msg) = actuator_rx.dequeue() {
            let _ = actuator.handle(msg);
        }

        // Example: Send messages based on sensor readings
        if sensor.last_reading > sensor.threshold {
            let _ = actuator_tx.enqueue(ActuatorMessage::TurnOn);
        }
    }
}
