//! Example: Handling External Event Enums
//!
//! This example demonstrates different patterns for working with event enums
//! that are defined in external crates and cannot be directly annotated with
//! the `#[statechart_event]` attribute.

#[allow(unused_imports)] // Needed for StateMachine trait methods
use lit_bit_core::StateMachine;
use lit_bit_macro::{statechart, statechart_event};

// Simulate an external crate's event enum that we cannot modify
mod external_crate {
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum ExternalEvent {
        Start,
        Stop,
        Configure { setting: u32 },
    }
}

// Pattern 1: Wrapper Enum
// Create a local enum that mirrors the external one
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[statechart_event]
pub enum WrappedEvent {
    #[default]
    Start,
    Stop,
    Configure {
        setting: u32,
    },
}

impl From<external_crate::ExternalEvent> for WrappedEvent {
    fn from(event: external_crate::ExternalEvent) -> Self {
        match event {
            external_crate::ExternalEvent::Start => WrappedEvent::Start,
            external_crate::ExternalEvent::Stop => WrappedEvent::Stop,
            external_crate::ExternalEvent::Configure { setting } => {
                WrappedEvent::Configure { setting }
            }
        }
    }
}

// Pattern 2: Newtype Wrapper
// Wrap the entire external enum in a newtype
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[statechart_event]
pub enum NewtypeEvent {
    External(external_crate::ExternalEvent),
    // Can add local events too
    #[default]
    Reset,
}

// Pattern 3: Hybrid Approach
// Decompose external events into local variants for better pattern matching
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[statechart_event]
pub enum HybridEvent {
    // Flatten the external events we care about
    #[default]
    Start,
    Stop,
    Configure {
        setting: u32,
    },
    // Group other external events if needed
    // OtherExternal(external_crate::ExternalEvent), // Removed for simplicity
    // Add local events
    Reset,
    Shutdown,
}

// Example state machine context
#[derive(Debug, Clone, Default)]
pub struct SystemContext {
    pub is_configured: bool,
    pub setting_value: u32,
    pub operation_count: u32,
}

// Example with Pattern 1: Wrapper Enum
statechart! {
    name: WrapperMachine,
    context: SystemContext,
    event: WrappedEvent,
    initial: Idle,

    state Idle {
        on WrappedEvent::Start => Running;
        on Configure { setting: _ } => Idle [action configure_system];
    }

    state Running {
        entry: increment_count;
        on WrappedEvent::Stop => Idle;
        on Configure { setting: _ } => Running [action configure_system];
    }
}

// Action functions
fn configure_system(ctx: &mut SystemContext, event: &WrappedEvent) {
    if let WrappedEvent::Configure { setting } = event {
        ctx.is_configured = true;
        ctx.setting_value = *setting;
        #[cfg(feature = "std")]
        println!("System configured with setting: {setting}");
    }
}

fn increment_count(ctx: &mut SystemContext, _event: &WrappedEvent) {
    ctx.operation_count += 1;
    #[cfg(feature = "std")]
    println!("Operation count: {}", ctx.operation_count);
}

// Helper function to demonstrate usage
#[allow(dead_code)]
/// Processes an external event by converting it to a wrapped event and sending it to the state machine.
///
/// Converts an `external_crate::ExternalEvent` into a `WrappedEvent`, sends it to the provided `WrapperMachine`, and handles the result by printing status messages if the `std` feature is enabled.
///
/// # Examples
///
/// ```
/// let mut machine = WrapperMachine::new(SystemContext::default(), WrappedEvent::Start);
/// process_external_event(&mut machine, external_crate::ExternalEvent::Configure { setting: 42 });
/// ```
fn process_external_event(
    machine: &mut WrapperMachine,
    external_event: external_crate::ExternalEvent,
) {
    // Convert external event to wrapped event
    let wrapped = WrappedEvent::from(external_event);
    match machine.send(&wrapped) {
        lit_bit_core::SendResult::Transitioned => {
            #[cfg(feature = "std")]
            println!("  -> Event handled: transition occurred");
        }
        lit_bit_core::SendResult::NoMatch => {
            #[cfg(feature = "std")]
            println!("  -> Event ignored: no matching transition");
        }
        #[allow(clippy::used_underscore_binding)]
        lit_bit_core::SendResult::Error(_e) => {
            #[cfg(feature = "std")]
            eprintln!("  -> Error processing event: {_e:?}");
        }
    }
}

#[cfg(feature = "std")]
fn main() {
    println!("=== External Event Handling Example ===\n");

    // Initialize the state machine
    let mut machine = WrapperMachine::new(
        SystemContext::default(),
        &WrappedEvent::Start, // Initial event
    )
    .expect("Failed to create wrapper machine");

    println!("Initial state: {:?}", machine.state());
    println!("Initial context: {:?}\n", machine.context());

    // Simulate receiving external events
    let external_events = vec![
        external_crate::ExternalEvent::Configure { setting: 42 },
        external_crate::ExternalEvent::Start,
        external_crate::ExternalEvent::Configure { setting: 100 },
        external_crate::ExternalEvent::Stop,
    ];

    for ext_event in external_events {
        println!("Processing external event: {ext_event:?}");
        process_external_event(&mut machine, ext_event);
        println!("  -> State: {:?}", machine.state());
        println!("  -> Context: {:?}\n", machine.context());
    }

    println!("=== Pattern Comparison ===");
    println!("1. Wrapper Enum: Best for full control and pattern matching");
    println!("2. Newtype: Simpler but less ergonomic pattern matching");
    println!("3. Hybrid: Good balance when you only care about some variants");
}

// Dummy main for no_std targets
#[cfg(not(feature = "std"))]
fn main() {
    // This external_events example is intended for std environments
}
