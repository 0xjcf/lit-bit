# Rust-Statechart Public API Usage Showcase

This document illustrates how a user would typically interact with a statechart created by the `lit-bit` crate, assuming the crate and its `statechart!` macro are working according to the `Spec.md`.

```rust
// main.rs (or library using the statechart)

// Assume the crate is named `lit-bit`
use lit-bit::{statechart, StateMachine};

// 1. Define the Context struct and its methods (for guards/actions)
#[derive(Debug, Clone, Default)]
struct AgentCtx {
    activation_count: u32,
    last_error: Option<String>,
}

impl AgentCtx {
    fn can_start(&self) -> bool {
        println!("[Guard] Checking AgentCtx.can_start (activation_count: {})", self.activation_count);
        self.activation_count < 5
    }

    fn start_up(&mut self) {
        self.activation_count += 1;
        println!("[Action] Executing AgentCtx.start_up (new count: {})", self.activation_count);
    }

    fn shut_down(&mut self) {
        println!("[Action] Executing AgentCtx.shut_down");
    }

    fn log_error(&mut self, error_message: &str) {
        println!("[Action] Logging error: {}", error_message);
        self.last_error = Some(error_message.to_string());
    }
}

// 2. Define the Events enum for the statechart
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentEvent {
    Activate,
    Deactivate,
    ReportError,
    Timeout, // For an `after` transition
}

// 3. Define the Statechart using the macro
// This macro call is expected to generate:
//   - A struct `Agent` (from `name: Agent`).
//   - An enum `AgentState` (e.g., `AgentState::Idle`, `AgentState::Active`, etc.).
//   - Implementation of `StateMachine` for `Agent`.
//   - A `Agent::new(context: AgentCtx) -> Self` constructor.
//   - A `agent_instance.matches(AgentState::Variant) -> bool` helper.
statechart! {
    name: Agent,
    context: AgentCtx,
    initial: Idle,

    state Idle {
        on Activate [guard .can_start] => Active [action .start_up];
        on ReportError => Errored [action .log_error("System fault from Idle")];
        after 5s => Active [action .start_up]; // Example: auto-activate if idle for too long
    }

    state Active {
        on Deactivate => Idle [action .shut_down];
        on ReportError => Errored [action .log_error("Critical issue while Active")];
        // Self-transition example
        on Activate [guard .can_start] => Active [action .start_up]; // Re-activate, maybe refresh something
    }

    state Errored {
        // Terminal-like state for this example; could have transitions out
        on Deactivate => Idle; // Allow deactivation from error state
    }
}

// --- Usage Example (typically in a function or main) ---
fn run_agent_lifecycle() {
    println!("--- Statechart Usage Example ---");

    let initial_context = AgentCtx::default();
    let mut agent_fsm = Agent::new(initial_context); // Macro-generated constructor

    assert!(agent_fsm.matches(AgentState::Idle)); // Macro-generated helper
    println!("Initial state: {:?} (context: {:?})", agent_fsm.state(), agent_fsm.context());

    // Scenario 1: Successful activation
    println!("\nSending Activate event (attempt 1)...");
    if agent_fsm.send(AgentEvent::Activate) {
        println!("Event processed. New state: {:?}", agent_fsm.state());
        assert!(agent_fsm.matches(AgentState::Active));
        assert_eq!(agent_fsm.context().activation_count, 1);
    } else {
        println!("Event ignored (guard likely failed or no transition). State: {:?}", agent_fsm.state());
    }
    println!("Current context: {:?}", agent_fsm.context());

    // Scenario 2: Deactivation
    println!("\nSending Deactivate event...");
    if agent_fsm.send(AgentEvent::Deactivate) {
        println!("Event processed. New state: {:?}", agent_fsm.state());
        assert!(agent_fsm.matches(AgentState::Idle));
    } else {
        println!("Event ignored. State: {:?}", agent_fsm.state());
    }
    println!("Current context: {:?}", agent_fsm.context());

    // Scenario 3: Event ignored in current state
    println!("\nSending Deactivate event again (while already Idle)...");
    if agent_fsm.send(AgentEvent::Deactivate) { // Should be false if no transition for Deactivate in Idle
        println!("Event processed. New state: {:?}", agent_fsm.state());
    } else {
        println!("Event correctly ignored. State remains: {:?}", agent_fsm.state());
        assert!(agent_fsm.matches(AgentState::Idle));
    }

    // Scenario 4: Transition to Errored state
    println!("\nSending ReportError event...");
    agent_fsm.send(AgentEvent::Activate); // Get back to Active for this test
    println!("Current state before error: {:?}", agent_fsm.state());

    if agent_fsm.send(AgentEvent::ReportError) {
        println!("Event processed. New state: {:?}", agent_fsm.state());
        assert!(agent_fsm.matches(AgentState::Errored));
        assert!(agent_fsm.context().last_error.is_some());
        println!("Error logged: {}", agent_fsm.context().last_error.as_ref().unwrap());
    } else {
        println!("Event ignored. State: {:?}", agent_fsm.state());
    }

    // Scenario 5: Guard prevents transition (try to activate 5 more times)
    println!("\nAttempting to exceed max activations...");
    agent_fsm.send(AgentEvent::Deactivate); // Back to Idle
    for i in 0..5 { // Already activated once or twice, so a few more should hit the guard
        println!("Sending Activate event (attempt {})...", i + 2);
        let processed = agent_fsm.send(AgentEvent::Activate);
        if !processed && agent_fsm.matches(AgentState::Idle) {
            println!("Activation guard .can_start likely prevented transition (count: {}).", agent_fsm.context().activation_count);
            break;
        }
        // Go back to Idle to try activating again
        if agent_fsm.matches(AgentState::Active) {
            agent_fsm.send(AgentEvent::Deactivate);
        }
    }
    assert!(agent_fsm.context().activation_count <= 5);
    println!("Final context after activation attempts: {:?}", agent_fsm.context());

    println!("\n--- Example Finished ---");
}

// To make it runnable (conceptually)
fn main() {
    run_agent_lifecycle();
}

```

This example demonstrates:
1. Defining context and event types.
2. Using the `statechart!` macro.
3. Instantiating the generated state machine.
4. Sending events using `your_machine.send(YourEvent::Variant)`.
5. Checking the current state using `your_machine.state()` and the ergonomic `your_machine.matches(YourState::Variant)`.
6. Accessing context data via `your_machine.context()`.
7. How guards and actions (defined on the context struct) are invoked. 