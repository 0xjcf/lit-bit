# Rust-Statechart Specification (v0.1 - DRAFT)

> **Purpose**: This document specifies the `rust-statechart` library, including its core concepts, macro grammar, public API, and intended behavior. It serves as the source of truth for Phase 0 and beyond.

---

## Table of Contents

1.  [Introduction](#1-introduction)
    1.1. [Goals](#11-goals)
    1.2. [Non-Goals](#12-non-goals)
    1.3. [Core Concepts](#13-core-concepts)
2.  [Macro Grammar (`statechart!`)](#2-macro-grammar-statechart)
3.  [Public API](#3-public-api)
    3.1. [`StateMachine` Trait](#31-statemachine-trait)
    3.2. [State & Event Enums](#32-state--event-enums)
    3.3. [Context Data](#33-context-data)
4.  [Behavior & Semantics](#4-behavior--semantics)
    4.1. [State Transitions](#41-state-transitions)
    4.2. [Entry/Exit Actions](#42-entryexit-actions)
    4.3. [Guards](#43-guards)
    4.4. [Hierarchy (Nested States)](#44-hierarchy-nested-states)
    4.5. [Parallel States](#45-parallel-states)
    4.6. [Delayed Transitions / Timers](#46-delayed-transitions--timers)
    4.7. [Invoked Services / Child Statecharts](#47-invoked-services--child-statecharts)
    4.8. [History States (TBD)](#48-history-states-tbd)
5.  [Error Handling](#5-error-handling)
    5.1. [Compile-Time Errors](#51-compile-time-errors)
    5.2. [Runtime Errors/Panics](#52-runtime-errorspanics)
6.  [Feature Flags](#6-feature-flags)
    6.1. [`std`](#61-std)
    6.2. [`async`](#62-async)
    6.3. [`diagram`](#63-diagram)
7.  [Actor Model (Optional)](#7-actor-model-optional)
8.  [Future Considerations (Post v0.1)](#8-future-considerations-post-v01)

---

## 1. Introduction

_Brief overview of the library, its purpose, and the problems it aims to solve. Inspired by XState but tailored for Rust's strengths (type safety, performance, `no_std`)._

### 1.1. Goals

*   Ergonomic, declarative statechart definition via a procedural macro.
*   Type-safe states, events, and transitions.
*   `#![no_std]` compatibility by default for embedded systems.
*   Minimal binary footprint.
*   High performance for event processing.
*   Support for Harel statecharts (hierarchy, parallel regions, history (TBD)).
*   Optional actor model integration (`Mailbox`, `Actor` trait).
*   Clear compile-time error messages for invalid chart definitions.

### 1.2. Non-Goals (for v0.1)

*   Full XState compatibility (some features may be Rust-idiomatic or deferred).
*   Visual editor or GUI tooling (focus on library core).
*   Automatic interpretation of SCXML.
*   Distributed statecharts.

### 1.3. Core Concepts

*   **Statechart**: A specification of system behavior.
*   **State**: A condition in which a system can be.
    *   **Atomic State**: A state with no substates.
    *   **Compound State**: A state with substates (child states).
    *   **Parallel State**: A compound state whose child states are active concurrently.
    *   **Final State**: A state that indicates the completion of its parent state's behavior.
*   **Event**: An occurrence that can trigger a state transition.
*   **Transition**: A change from one state to another, triggered by an event.
*   **Action**: An executable piece of code performed upon state entry, exit, or during a transition. Can be a reference to a method on the context (e.g., `.my_action_method`).
*   **Guard (Condition)**: A boolean predicate that must be true for a transition to occur. Can be a reference to a method on the context (e.g., `.my_guard_method`).
*   **Context**: Data storage associated with the statechart instance.
*   **Delayed Transition (Timer)**: A transition that occurs after a specified duration if the state remains active.
*   **Invoked Service / Child Statechart**: A statechart can invoke or spawn other services or child statecharts, managing their lifecycle and communication.

---

## 2. Macro Grammar (`statechart!`)

_This section will define the EBNF (or similar formal notation) for the `statechart!` macro. It should align with `.cursor/rules/statechart.mdc` §3, but is expanded here to reflect richer syntax._

```ebnf
statechart    ::= 'statechart!' '{'
                    header_field+
                    state_definition* // States can be defined directly at the top level
                  '}'

header_field  ::= 'name:' IDENT ','
                | 'initial:' IDENT ','
                | 'context:' TYPE ','

// Placeholder based on statechart.mdc, adapted and expanded:
// states        ::= state_definition* // No longer a separate 'states:' block
state_definition ::= 'state' IDENT state_attributes? '{' state_body_item* '}'

state_attributes ::= '[' attribute (',' attribute)* ']'
attribute        ::= 'parallel'
                   // Other future attributes like 'history', 'final'

state_body_item  ::= 'on' IDENT transition_guard? '=>' IDENT transition_action? ';'
                   | 'after' DURATION '=>' IDENT transition_action? ';'
                   | 'invoke' 'child' IDENT '->' statechart_invocation ';' // Simplified for now
                   | 'entry' '=>' action_reference ';' // Optional entry action
                   | 'exit' '=>' action_reference ';'  // Optional exit action
                   | 'initial:' IDENT ';' // For compound states
                   | state_definition // For nested states
                   // | 'region' IDENT '{' ... '}' // For named regions in parallel states, if needed

transition_guard ::= '[' 'guard' guard_reference ']'
guard_reference  ::= '.' IDENT // Method on context

transition_action::= '[' 'action' action_reference ']'
action_reference ::= '.' IDENT // Method on context

DURATION         ::= NUMBER ('ms' | 's' | 'm' | 'h') // e.g., 5s, 500ms

IDENT            ::= /* a Rust identifier */
TYPE             ::= /* a Rust type identifier */
NUMBER           ::= /* a numeric literal */
statechart_invocation ::= /* syntax for invoking another statechart, potentially another macro call */
```

---

## 3. Public API

### 3.1. `StateMachine` Trait

_Define the core trait that all generated statecharts will implement. This trait provides the fundamental methods for interacting with a state machine instance._

```rust
pub trait StateMachine {
    /// The type representing the states of this state machine, typically an enum.
    /// Must be comparable, cloneable, and debuggable.
    type State: Copy + Clone + PartialEq + core::fmt::Debug;

    /// The type representing the events that can be sent to this state machine, typically an enum.
    /// Must be comparable, cloneable, and debuggable.
    type Event: Copy + Clone + PartialEq + core::fmt::Debug;

    /// The type for the context data associated with this state machine.
    type Context;

    /// Sends an event to the state machine, potentially causing state transitions and actions.
    ///
    /// # Arguments
    /// * `event`: The event to process.
    ///
    /// # Returns
    /// * `true` if the event resulted in one or more transitions (including self-transitions).
    /// * `false` if the event was ignored (e.g., no matching transition for the current state, or a guard condition prevented the transition).
    fn send(&mut self, event: Self::Event) -> bool;

    /// Returns the current active state of the state machine.
    fn state(&self) -> Self::State;

    /// Returns an immutable reference to the state machine's context data.
    fn context(&self) -> &Self::Context;

    /// Returns a mutable reference to the state machine's context data.
    /// This allows actions or other external logic to modify the context.
    fn context_mut(&mut self) -> &mut Self::Context;

    // Note: For ergonomic state checking, the `statechart!` macro will typically generate
    // a `matches(&self, state: Self::State) -> bool` method on the concrete state machine struct.
    // Example: `if my_machine.matches(MyState::Active) { /* ... */ }`
    // This helper is not part of the core `StateMachine` trait itself to keep the trait minimal,
    // but is a convention for generated code.
}
```

### 3.2. State & Event Enums

_The macro will generate enums for states and events based on the definition._

*Example:*
```rust
// From a definition like: states: { Green, Yellow, Red }, events: { Timer, PowerOutage }
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TrafficLightState {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TrafficLightEvent {
    Timer,
    PowerOutage,
}
```

### 3.3. Context Data

_How context data is defined in the macro and accessed/modified._

---

## 4. Behavior & Semantics

_Detailed explanation of how the statechart operates._

### 4.1. State Transitions
_Event dispatch order, transition selection, internal vs. external transitions._

### 4.2. Entry/Exit Actions
_Order of execution, parameters, error handling._

### 4.3. Guards
_Evaluation timing, access to context/event data._

### 4.4. Hierarchy (Nested States)
_Event bubbling, initial states of compound states, parent/child relationships._

### 4.5. Parallel States
_Region activation/deactivation, event processing in parallel regions. Defined using the `[parallel]` attribute on a state._

### 4.6. Delayed Transitions / Timers
_Transitions triggered by the passage of time. Defined using the `after DURATION => TARGET_STATE [action .optional_action];` syntax within a state body. When a state with an `after` transition is entered, an internal timer is started. If the state is exited before the timer fires, the timer is cancelled. If the timer fires, the specified transition occurs._

### 4.7. Invoked Services / Child Statecharts
_A state can invoke other services or child statecharts. This is defined using the `invoke child SERVICE_NAME -> statechart!(...);` syntax (actual invocation mechanism TBD). The parent statechart can send events to and receive events from the invoked child. The lifecycle of the child (start, stop) is typically tied to the parent state's entry and exit._

### 4.8. History States (TBD)
_Shallow vs. deep history, default transitions._

---

## 5. Error Handling

### 5.1. Compile-Time Errors
_List of errors the macro should detect (e.g., unknown state, duplicate transition, unreachable region). Reference `statechart.mdc`._

### 5.2. Runtime Errors/Panics
_When (if ever) the runtime component might panic. Prefer `Result` types in `std` builds._

---

## 6. Feature Flags

_As defined in `statechart.mdc` and `ROADMAP.md`._

### 6.1. `std`
_Enables Tokio mailbox, file I/O for diagrams, etc._

### 6.2. `async`
_Pulls `alloc`, `futures`, `async-trait`. Allows `async fn` in actions/guards._

### 6.3. `diagram`
_Emits `TRANSITIONS` table, formatters for DOT/Mermaid. Off in firmware builds._

---

## 7. Actor Model

_Brief description of how the `Actor` and `Mailbox` traits will integrate if the `std` or `async` features are enabled. Focus on single-threaded execution guarantee and back-pressure._

---

## 8. Future Considerations (Post v0.1)

_Ideas for v0.2 and beyond (e.g., statechart inspection API, advanced testing utilities, SCXML import/export if demand exists)._
