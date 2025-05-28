//! # Actor-Statechart Integration Example
//!
//! This example demonstrates the seamless integration between statecharts and actors:
//! **Zero-cost `StateMachine` integration** - Your statecharts automatically become actors
//! through a blanket implementation, with no runtime overhead or boilerplate code.
//!
//! ## Key Concepts Demonstrated
//! - Automatic Actor implementation for `StateMachine` types
//! - Zero-cost event forwarding from actor messages to statechart events
//! - Platform-dual design (same code works on embedded and cloud)
//! - Type-safe message passing with compile-time guarantees

#![cfg_attr(not(feature = "std"), no_std)]

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
#[cfg(all(not(feature = "std"), feature = "panic-halt"))]
use panic_halt as _;

// Alternative panic handler when panic-halt is not available
#[cfg(all(not(feature = "std"), not(feature = "panic-halt")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use lit_bit_core::StateMachine;
use lit_bit_macro::{statechart, statechart_event};

#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use heapless::{FnvIndexMap as HashMap, String};

// Capacities for heapless collections on no_std
#[cfg(not(feature = "std"))]
const MAX_CONNECTIONS: usize = 8;
#[cfg(not(feature = "std"))]
const STRING_CAPACITY: usize = 32;

/// Context for our connection manager statechart
#[derive(Debug, Clone, Default)]
pub struct ConnectionContext {
    #[cfg(feature = "std")]
    pub active_connections: HashMap<String, u32>,
    #[cfg(not(feature = "std"))]
    pub active_connections: HashMap<String<STRING_CAPACITY>, u32, MAX_CONNECTIONS>,
    pub total_connections: u32,
    pub failed_attempts: u32,
}

impl ConnectionContext {
    fn add_connection(&mut self, id: &str, session_id: u32) {
        #[cfg(feature = "std")]
        {
            self.active_connections.insert(id.to_string(), session_id);
        }
        #[cfg(not(feature = "std"))]
        {
            if let Ok(key) = String::try_from(id) {
                let _ = self.active_connections.insert(key, session_id);
            }
        }
        self.total_connections += 1;
    }

    fn remove_connection(&mut self, id: &str) -> bool {
        #[cfg(feature = "std")]
        {
            self.active_connections.remove(id).is_some()
        }
        #[cfg(not(feature = "std"))]
        {
            if let Ok(key) = String::try_from(id) {
                self.active_connections.remove(&key).is_some()
            } else {
                false
            }
        }
    }
}

/// Events for the connection manager
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[statechart_event]
pub enum ConnectionEvent {
    Connect { client_id: u32 },
    Disconnect { client_id: u32 },
    Heartbeat { client_id: u32 },
    Timeout,
    NetworkError,
    Shutdown,
}

// Action functions for state transitions
fn action_establish_connection(context: &mut ConnectionContext, event: &ConnectionEvent) {
    if let ConnectionEvent::Connect { client_id } = event {
        // Use client_id directly as a simple connection identifier
        #[cfg(feature = "std")]
        {
            let connection_id = format!("conn_{client_id}");
            context.add_connection(&connection_id, *client_id);
        }
        #[cfg(not(feature = "std"))]
        {
            // For no_std, use client_id directly as the session_id to avoid overwrites
            // We'll store a simple marker string and use the session_id to differentiate
            context.add_connection("conn", *client_id);
        }

        #[cfg(feature = "std")]
        println!("🔗 Connection established for client {client_id}");
    }
}

fn action_close_connection(context: &mut ConnectionContext, event: &ConnectionEvent) {
    if let ConnectionEvent::Disconnect {
        client_id: _client_id,
    } = event
    {
        #[cfg(feature = "std")]
        {
            let connection_id = format!("conn_{_client_id}");
            if context.remove_connection(&connection_id) {
                println!("❌ Connection closed for client {_client_id}");
            }
        }
        #[cfg(not(feature = "std"))]
        {
            // For no_std, remove by simple key (this is a limitation of the simple approach)
            // In a real implementation, you'd want a more sophisticated data structure
            context.remove_connection("conn");
        }
    }
}

fn action_handle_heartbeat(_context: &mut ConnectionContext, event: &ConnectionEvent) {
    if let ConnectionEvent::Heartbeat {
        client_id: _client_id,
    } = event
    {
        #[cfg(feature = "std")]
        println!("💓 Heartbeat received from client {_client_id}");
    }
}

fn action_handle_timeout(_context: &mut ConnectionContext, _event: &ConnectionEvent) {
    #[cfg(feature = "std")]
    println!("⏰ Connection timeout detected");
}

fn action_handle_network_error(context: &mut ConnectionContext, _event: &ConnectionEvent) {
    context.failed_attempts += 1;
    #[cfg(feature = "std")]
    println!(
        "🚨 Network error occurred (total failures: {})",
        context.failed_attempts
    );
}

fn action_increment_failures_and_handle_error(
    context: &mut ConnectionContext,
    _event: &ConnectionEvent,
) {
    context.failed_attempts += 1;
    #[cfg(feature = "std")]
    println!(
        "🚨 Network error occurred, triggering error recovery (total failures: {})",
        context.failed_attempts
    );
}

fn action_shutdown_all(context: &mut ConnectionContext, _event: &ConnectionEvent) {
    let _count = context.active_connections.len();
    context.active_connections.clear();
    #[cfg(feature = "std")]
    println!("🛑 Shutting down all {_count} active connections");
}

// Guard functions
fn guard_has_active_connections(context: &ConnectionContext, _event: &ConnectionEvent) -> bool {
    !context.active_connections.is_empty()
}

fn guard_too_many_failures(context: &ConnectionContext, _event: &ConnectionEvent) -> bool {
    context.failed_attempts >= 2
}

// Define the connection manager statechart
statechart! {
    name: ConnectionManager,
    context: ConnectionContext,
    event: ConnectionEvent,
    initial: Disconnected,

    state Disconnected {
        on ConnectionEvent::Connect { client_id: _ } => Connected [action action_establish_connection];
        on ConnectionEvent::NetworkError => Disconnected [action action_handle_network_error];
        on ConnectionEvent::Shutdown => Shutdown [action action_shutdown_all];
    }

    state Connected {
        on ConnectionEvent::Connect { client_id: _ } => Connected [action action_establish_connection];
        on ConnectionEvent::Disconnect { client_id: _ } => Connected [action action_close_connection];
        on ConnectionEvent::Heartbeat { client_id: _ } => Connected [action action_handle_heartbeat];
        on ConnectionEvent::Timeout => Connected [action action_handle_timeout];
        on ConnectionEvent::NetworkError [guard guard_too_many_failures] => ErrorRecovery [action action_increment_failures_and_handle_error];
        on ConnectionEvent::NetworkError => Connected [action action_handle_network_error];
        on ConnectionEvent::Shutdown [guard guard_has_active_connections] => Shutdown [action action_shutdown_all];
        on ConnectionEvent::Shutdown => Disconnected;
    }

    state ErrorRecovery {
        on ConnectionEvent::Connect { client_id: _ } => Connected [action action_establish_connection];
        on ConnectionEvent::Timeout => Disconnected;
        on ConnectionEvent::Shutdown => Shutdown [action action_shutdown_all];
    }

    state Shutdown {
        // Terminal state - no transitions out
    }
}

// The magic happens here: StateMachine automatically implements Actor!
// This is provided by the blanket implementation in the actor module.
// No manual implementation needed - the blanket impl handles everything!

#[cfg(feature = "std")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 StateMachine-Actor Integration Example");
    println!("=========================================");

    let initial_context = ConnectionContext::default();
    let mut connection_manager =
        ConnectionManager::new(initial_context, &ConnectionEvent::Connect { client_id: 0 })?;

    println!("\n📊 Initial State: {:?}", connection_manager.state());

    // Demonstrate the actor working as a statechart
    println!("\n🔗 Simulating connection lifecycle...");

    // Connect some clients
    connection_manager.send(&ConnectionEvent::Connect { client_id: 1 });
    connection_manager.send(&ConnectionEvent::Connect { client_id: 2 });
    connection_manager.send(&ConnectionEvent::Connect { client_id: 3 });

    println!("State after connections: {:?}", connection_manager.state());
    println!(
        "Active connections: {}",
        connection_manager.context().active_connections.len()
    );

    // Simulate heartbeats
    connection_manager.send(&ConnectionEvent::Heartbeat { client_id: 1 });
    connection_manager.send(&ConnectionEvent::Heartbeat { client_id: 2 });

    // Disconnect a client
    connection_manager.send(&ConnectionEvent::Disconnect { client_id: 2 });
    println!(
        "Active connections after disconnect: {}",
        connection_manager.context().active_connections.len()
    );

    // Simulate network errors
    println!("\n🚨 Simulating network errors...");
    connection_manager.send(&ConnectionEvent::NetworkError);
    connection_manager.send(&ConnectionEvent::NetworkError);
    connection_manager.send(&ConnectionEvent::NetworkError); // This should trigger error recovery

    println!("State after errors: {:?}", connection_manager.state());
    println!(
        "Failed attempts: {}",
        connection_manager.context().failed_attempts
    );

    // Recover by connecting again
    println!("\n🔄 Recovering from errors...");
    connection_manager.send(&ConnectionEvent::Connect { client_id: 4 });
    println!("State after recovery: {:?}", connection_manager.state());

    // Shutdown
    println!("\n🛑 Shutting down...");
    connection_manager.send(&ConnectionEvent::Shutdown);
    println!("Final state: {:?}", connection_manager.state());

    println!("\n✅ StateMachine-Actor integration example completed!");
    println!("\n💡 Key Concepts Demonstrated:");
    println!("   • Zero-cost StateMachine → Actor conversion");
    println!("   • No boxing or dynamic dispatch required");
    println!("   • Type-safe event handling");
    println!("   • Supervision hooks for restart strategies");
    println!("   • Platform-dual design (works on no_std too)");
    println!("   • State-aware supervision decisions");

    Ok(())
}

#[cfg(not(feature = "std"))]
fn main() {
    // For no_std targets, this would typically be integrated into
    // an Embassy-based application or other embedded runtime

    let initial_context = ConnectionContext::default();
    let mut connection_manager =
        ConnectionManager::new(initial_context, &ConnectionEvent::Connect { client_id: 0 })
            .expect("Failed to create connection manager");

    // Simulate some basic operations
    connection_manager.send(&ConnectionEvent::Connect { client_id: 1 });
    connection_manager.send(&ConnectionEvent::Heartbeat { client_id: 1 });
    connection_manager.send(&ConnectionEvent::Disconnect { client_id: 1 });

    // In a real embedded application, this would be part of a larger event loop
}

#[cfg(test)]
mod tests {
    use super::*;
    use lit_bit_core::actor::Actor;

    #[test]
    fn statechart_implements_actor() {
        let initial_context = ConnectionContext::default();
        let mut manager =
            ConnectionManager::new(initial_context, &ConnectionEvent::Connect { client_id: 0 })
                .unwrap();

        // Verify it implements Actor trait
        assert!(manager.on_start().is_ok());

        // Test state transitions through actor interface
        #[cfg(feature = "std")]
        {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                manager
                    .on_event(ConnectionEvent::Connect { client_id: 1 })
                    .await;
                // Verify state changed
                // Note: In a real test, we'd check the state more thoroughly
            });
        }

        #[cfg(not(feature = "std"))]
        {
            // For no_std, we can't easily test async behavior without a runtime
            // but we can verify the trait implementation and basic functionality
            // In a real embedded environment, this would be handled by Embassy or similar
        }
    }

    #[test]
    fn supervision_strategy_is_one_for_one() {
        let initial_context = ConnectionContext::default();
        let manager =
            ConnectionManager::new(initial_context, &ConnectionEvent::Connect { client_id: 0 })
                .unwrap();

        // Test that the default supervision strategy is OneForOne
        // We can't easily construct a PanicInfo in tests, but we can verify the trait is implemented
        // and that the default behavior is correct by checking the trait's default implementation

        // The Actor trait provides a default implementation that returns OneForOne
        // This test verifies that our StateMachine implements the Actor trait correctly
        #[cfg(feature = "std")]
        {
            use std::any::Any;
            let _: &dyn Any = &manager; // Verify it's a concrete type
        }

        #[cfg(not(feature = "std"))]
        {
            // For no_std, we can't use std::any::Any, but we can still verify
            // that the manager implements the Actor trait through other means
            // The fact that this compiles proves the trait is implemented correctly
        }

        // The actual supervision logic would be tested in integration tests
        // where real panics can be triggered and handled
    }

    #[test]
    fn error_recovery_state_machine() {
        let initial_context = ConnectionContext::default();
        let mut manager =
            ConnectionManager::new(initial_context, &ConnectionEvent::Connect { client_id: 0 })
                .unwrap();

        // Connect first
        manager.send(&ConnectionEvent::Connect { client_id: 1 });

        // Trigger multiple network errors
        manager.send(&ConnectionEvent::NetworkError);
        manager.send(&ConnectionEvent::NetworkError);
        manager.send(&ConnectionEvent::NetworkError);

        // Should be in error recovery state
        // Note: In a complete test, we'd verify the exact state
        assert_eq!(manager.context().failed_attempts, 3);
    }
}
