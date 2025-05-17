# Rust-Statechart Specification (v0.1 - DRAFT)

> **Purpose**: This document specifies the `lit-bit` library, including its core concepts, macro grammar, public API, and intended behavior. It serves as the source of truth for Phase 0 and beyond. The library is licensed under MIT OR Apache-2.0.
> **Last Updated**: 2025-07-27

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
7.  [Actor Model (Phase 4 Target)](#7-actor-model-phase-4-target)
8.  [Future Considerations (Post v0.1)](#8-future-considerations-post-v01)
9.  [Design Insights & Mitigation Strategies (2025-05 Research Audit)](#9-design-insights--mitigation-strategies-2025-05-research-audit)

---

## 1. Introduction

_Brief overview of the library, its purpose, and the problems it aims to solve. Inspired by XState but tailored for Rust's strengths (type safety, performance), focusing on `#![no_std]` compatibility by default and providing an optional actor model wrapper for concurrency._

### 1.1. Goals

*   Ergonomic, declarative statechart definition via a procedural macro.
*   Type-safe states, events, and transitions.
*   `#![no_std]` compatibility by default for embedded systems.
*   Minimal binary footprint.
*   High performance for event processing.
*   Support for Harel statecharts (hierarchy, parallel regions, history (TBD)).
*   Optional actor model integration (`Mailbox`, `Actor` trait).
*   Clear compile-time error messages for invalid chart definitions.
*   Automated diagram generation (DOT / Mermaid) behind `diagram` feature (Phase 8).

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
    *   **Final State**: A state that indicates the completion of its parent state's behavior. *(TBD post-v0.1)*
*   **Event**: An occurrence that can trigger a state transition.
*   **Transition**: A change from one state to another, triggered by an event.
*   **Action**: An executable piece of code performed upon state entry, exit, or during a transition. Can be a reference to a method on the context (e.g., `.my_action_method`).
*   **Guard (Condition)**: A boolean predicate that must be true for a transition to occur. Can be a reference to a method on the context (e.g., `.my_guard_method`).
*   **Context**: Data storage associated with the statechart instance.
*   **Delayed Transition (Timer)**: A transition that occurs after a specified duration if the state remains active.
*   **Invoked Service / Child Statechart**: A statechart can invoke or spawn other services or child statecharts, managing their lifecycle and communication.

---

## 2. Macro Grammar (`statechart!`)

_This section defines the EBNF grammar for the `statechart!` macro. This grammar specifies the syntax for defining state machines, including states, events, transitions, actions, guards, and other features._

```ebnf
statechart    ::= 'statechart!' '{'
                    // Header fields defining the overall machine
                    'name:' IDENT ','
                    'initial:' state_ref ','
                    'context:' TYPE ','

                    // State definitions (can be nested)
                    state_definition+
                  '}'

state_definition ::= 'state' state_ref state_attributes? '{'
                       state_body_item*
                     '}'

state_ref        ::= IDENT // Reference to a state name (e.g., 'Idle', 'Processing')

state_attributes ::= '[' attribute (',' attribute)* ']'
attribute        ::= 'parallel' // State contains parallel regions.
                   // | 'history' ('shallow' | 'deep')? // Future attribute
                   // | 'final' // Future attribute

state_body_item  ::= state_definition // Nested state
                   | 'initial:' state_ref ';' // Required for compound/parallel states
                   | 'entry' '=>' action_ref ';' // Action on entering this state
                   | 'exit' '=>' action_ref ';'  // Action on exiting this state
                   | 'on' event_ref transition_guard? '=>' transition_target transition_action? ';' // Event transition
                   | 'after' DURATION '=>' transition_target transition_action? ';' // Delayed transition
                   | 'invoke' invocation_details ';' // Invoke child machine/service

event_ref        ::= IDENT // Reference to an event name (e.g., 'Submit', 'Cancel')

// A transition_guard requires the 'guard' keyword to disambiguate from a potential action.
transition_guard ::= '[' 'guard' condition_ref ']'
condition_ref    ::= '.' IDENT // Method on context returning bool

transition_target::= state_ref // Target state for the transition
                   // | 'none' // Explicit internal transition (stay in state) - Deferred post-v0.1

transition_action::= '[' 'action' action_ref ']'
action_ref       ::= '.' IDENT // Method on context

DURATION         ::= NUMBER ('ms' | 's' | 'm' | 'h') // e.g., 5s, 500ms

invocation_details ::= /* Syntax for invoking children, see Phase 7 on roadmap */
                   // Example sketch: 'child' child_name ':' child_statechart_ref ('{' mapping* '}')?

IDENT            ::= /* A valid Rust identifier */
TYPE             ::= /* A valid Rust type path */
NUMBER           ::= /* A Rust integer literal */

// Note: Actions and guards reference methods defined on the `Context` struct.
// The method signature is inferred (e.g., guards take `&Context` and return `bool`, actions take `&mut Context`).
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
    /// _Note: This method is typically used directly in bare-metal or synchronous contexts. When using the Actor Model, events are usually sent via the mailbox (`try_send`/`send().await`)._
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
    /// but is a convention for generated code.
    ///
    /// # Thread Safety
    /// The generated state machine instance is typically `Send` but **not** `Sync`,
    /// as internal state transitions require mutable access. Access via the Actor Model
    /// ensures safe concurrent access.
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

The `Context` stores the quantitative or persistent data associated with the state machine instance. Actions and guards operate on this data.

1.  **Definition**: The type of the context data structure is specified in the `statechart!` macro header using the `context: TYPE,` field, where `TYPE` is a path to a Rust struct or tuple defined elsewhere.
    *   Example: `statechart! { name: TrafficLight, initial: Green, context: TrafficLightContext, ... }`
2.  **Data Structure**: The specified `TYPE` must be a concrete Rust struct or tuple. It typically contains fields relevant to the state machine's operation (e.g., counters, flags, user data, configuration).
    ```rust
    // Example context struct defined outside the macro
    pub struct TrafficLightContext {
        pub cycles: u32,
        pub emergency_mode: bool,
    }
    ```
3.  **Initialization**: An instance of the context struct must be provided when creating the state machine instance. The macro-generated constructor will typically take this initial context as an argument.
4.  **Access**:
    *   **Guards**: Guard methods defined on the context struct receive immutable access (`&Context`) via the `self` parameter.
    *   **Actions**: Entry, exit, and transition action methods defined on the context struct receive mutable access (`&mut Context`) via the `self` parameter, allowing them to modify the data.
      _(Performance Note: In embedded contexts, consider marking guard/action methods with `#[inline]` or `#[inline(always)]` if appropriate)._
5.  **Ownership**: The state machine instance owns the `Context` data. External access is provided via the `context()` and `context_mut()` methods defined in the `StateMachine` trait.
6.  **Serialization (Optional)**: If the `diagram` or other future features requiring serialization are enabled, the `Context` type may need to derive or implement `serde::Serialize` and `serde::Deserialize`. This is not required for core functionality.

---

## 4. Behavior & Semantics

_Detailed explanation of how the statechart operates._

### 4.1. State Transitions
_Event dispatch order, transition selection, internal vs. external transitions._

When an event is sent to the state machine via `send(event)`, the following process occurs to determine and execute transitions:

1.  **Event Matching**: The machine checks if the current active state configuration has any transitions defined for the received `event`. This check includes transitions defined on the current state(s) and any parent states (up to the root).
2.  **Guard Evaluation**: If a matching transition has a `guard` condition (`[.guard .my_guard_method]`), the referenced method on the `Context` is called (with `&Context` access). If the guard returns `false`, the transition is blocked, and the machine continues searching for other potential transitions for the same event (e.g., on parent states). If the guard returns `true`, the transition is selected. If there's no guard, it's implicitly `true`.
3.  **Transition Selection**:
    *   **Priority**: Transitions defined on deeper (child) states take priority over transitions defined on ancestor (parent) states for the same event.
    *   **First Match**: If multiple transitions are defined on the *same* state for the same event (e.g., with different guards), the *first* one defined in the macro whose guard evaluates to `true` is chosen.
    *   **No Match**: If no matching, unblocked transition is found in the current state or its ancestors, the event is considered unhandled, and the machine remains in its current state configuration. `send()` returns `false`.
4.  **Transition Execution**: If a transition is selected, `send()` will return `true`, and the following steps execute in order:
    *   **Exit Actions**: Execute exit actions (`exit => .action`) for all states being exited, starting from the deepest child state and moving upwards towards the least common ancestor (LCA) state of the source and target states.
    *   **Transition Actions**: Execute the action associated with the transition itself (`=> TARGET_STATE [action .action]`), if defined. This action receives `&mut Context`.
    *   **Enter Actions**: Execute entry actions (`entry => .action`) for all states being entered, starting from the state just below the LCA and moving downwards to the target state. If the target state is compound, its `initial:` state is entered recursively, triggering its entry actions as well.
5.  **Internal vs. External Transitions**:
    *   **External (Default)**: A transition from state `A` to state `B` (where `A != B`) causes `A` to be exited and `B` to be entered, triggering relevant exit/entry actions. A self-transition (`on EVENT => A`) also causes exit and entry actions for `A`.
    *   **Internal (Implicit)**: If a transition is handled by a *parent* state without specifying a target state change for the child, only the parent's transition action (if any) runs. The child state remains active, and its exit/entry actions are *not* executed. (Explicit syntax for internal transitions, like `target: none`, might be added later if needed for clarity).
    *   **Parallel State Override Example**: If a parallel state `P` has regions `R1` and `R2`, and an event `E` arrives: If `P` defines `on E => TargetState;` and `R1` (currently active) also defines `on E => R1_Target;`, the transition defined on `P` takes precedence. `R1` and `R2` (and their children) will be exited, `P` will be exited, and `TargetState` will be entered. The transition in `R1` is ignored because the event was handled by the parent `P`.

### 4.2. Entry/Exit Actions
_Order of execution, parameters, error handling._

Entry and exit actions allow the state machine to perform side effects when states are entered or exited. They are defined using the `entry => .action_name` and `exit => .action_name` syntax within a state definition.

1.  **Purpose**:
    *   **Entry Actions**: Typically used for setup tasks specific to the state being entered (e.g., starting timers, initializing state-local data, sending commands).
    *   **Exit Actions**: Typically used for cleanup tasks specific to the state being exited (e.g., stopping timers, clearing state-local data, finalizing operations).
2.  **Execution Order**: As detailed in *4.1 State Transitions*, during a transition:
    *   Exit actions of the source state (and its children, if compound) are executed first, from deepest child upwards.
    *   Entry actions of the target state (and its children, if compound and entering its initial state) are executed last, from the highest parent being entered downwards to the final target state(s).
3.  **Context Access**: Both entry and exit action methods defined on the `Context` struct receive mutable access (`&mut Context`) allowing them to modify the machine's context data.
4.  **Idempotency**: Actions should ideally be designed to be idempotent, especially if error recovery or complex scenarios might lead to re-entry or repeated exits, although the core execution model guarantees execution only once per standard transition.
5.  **Error Handling**: Actions are expected to complete successfully. If an action needs to signal a failure, it should typically do so by modifying the context or potentially enqueueing a failure event for the state machine to process in a subsequent step. Direct panicking within actions is discouraged, especially in `no_std` environments or release builds. (`#![forbid(panic)]` might be enforced in some profiles).

### 4.3. Guards
_Evaluation timing, access to context/event data._

Guards (or conditions) determine whether a potential transition, triggered by an event, should actually be taken. They are defined using the `[.guard .condition_name]` syntax attached to an `on EVENT` transition.

1.  **Purpose**: To add conditional logic to transitions based on the current state of the `Context` or potentially properties of the triggering `Event` (if the guard method signature accepts it, TBD). Guards allow multiple transitions for the same event from the same state, each leading to a different target state or action based on specific conditions.
2.  **Evaluation Timing**: Guards are evaluated *after* an event matches a transition defined on a state (or its ancestors) but *before* any exit actions, transition actions, or entry actions are executed. See *4.1 State Transitions* Step 2.
3.  **Context Access**: Guard methods defined on the `Context` struct receive immutable access (`&Context`) because they should be side-effect free; their sole purpose is to return `true` or `false`. They must not modify the context.
4.  **Return Value**: A guard method must return `bool`. If it returns `true`, the transition is allowed to proceed (assuming no higher-priority transition was also triggered). If it returns `false`, the transition is blocked, and the event processing might continue searching for other valid transitions (e.g., on parent states).
5.  **Absence of Guard**: If a transition definition does not include a `[.guard ...]`, it is considered to have a guard that always returns `true`.
6.  **Multiple Guards**: If multiple transitions are defined on the *same state* for the *same event* but with different guards, they are evaluated in the order they appear in the macro definition. The first one whose guard returns `true` is selected.

### 4.4. Hierarchy (Nested States)
_Event bubbling, initial states of compound states, parent/child relationships._

Statecharts can organize states hierarchically, creating parent-child relationships. A state containing other state definitions is called a **compound state**.

1.  **Definition**: A compound state is defined by nesting `state ... {}` definitions within another `state ... {}` block.
2.  **Initial State**: Every compound state *must* declare an initial substate using the `initial: SUBSTATE_NAME;` syntax within its definition block. When the state machine transitions into a compound state, it automatically enters this declared initial substate (triggering the initial substate's entry actions, if any).
3.  **Event Bubbling**: When an event occurs, if the currently active child state does not define a transition for that event (or its guards block it), the event "bubbles up" to its parent compound state. The parent state is then checked for transitions matching the event. This bubbling continues up the hierarchy until a state handles the event or the root of the statechart is reached.
4.  **Transition Priority**: As mentioned in *4.1 State Transitions*, transitions defined on child states have higher priority than transitions defined on parent states for the same event. The event is first checked against the innermost active state, and only bubbles up if unhandled.
5.  **Entering/Exiting Compound States**:
    *   **Entering**: When transitioning *into* a compound state `P` targeting its initial substate `C`, entry actions execute from `P` downwards to `C` (and further down if `C` is also compound). See *4.1 Transition Execution*.
    *   **Exiting**: When transitioning *out of* a substate `C` within a compound state `P` to a state outside of `P`, exit actions execute from `C` upwards to `P`. See *4.1 Transition Execution*.
    *   **Transitions within Compound State**: If a transition occurs between two substates `C1` and `C2` both directly within the same compound parent `P`, only `C1`'s exit actions and `C2`'s entry actions (and the transition action) are executed. `P`'s exit/entry actions are *not* executed because the machine remains within `P`.

### 4.5. Parallel States
_Region activation/deactivation, event processing in parallel regions. Defined using the `[parallel]` attribute on a state._

Parallel states allow a state machine to be in multiple orthogonal (independent) child states simultaneously. This is useful for modeling components that operate concurrently.

1.  **Definition**: A state is declared as parallel by adding the `[parallel]` attribute to its definition: `state MyParallelState [parallel] { ... }`.
2.  **Regions**: A parallel state must contain **two or more** direct child state definitions (effectively, regions). These regions are active concurrently. If a state is marked `[parallel]`, it cannot be an atomic state; it must define these regions. Unlike compound states which have only one active child state at a time, a parallel state has *all* of its direct child regions active concurrently. Each region is itself a standard state (atomic or compound) with its own initial state (if compound), transitions, etc.
    *   Example:
        ```rust
        state Parent [parallel] {
            initial: // Not applicable for parallel state itself
            state RegionA { initial: A1; state A1 {} state A2 {} }
            state RegionB { initial: B1; state B1 {} state B2 {} }
        }
        ```
3.  **Entering a Parallel State**: When a transition targets a parallel state `P`:
    *   The entry action of `P` (if any) is executed.
    *   Then, *all* direct child regions (`RegionA`, `RegionB`, etc.) are entered simultaneously. This means the `initial:` state for *each* region is entered, triggering their respective entry actions according to hierarchy (e.g., `RegionA` entry, then `A1` entry; `RegionB` entry, then `B1` entry). The exact order of execution *between* orthogonal regions' entry actions is generally not guaranteed and should not be relied upon.
4.  **Exiting a Parallel State**: When a transition leads *out* of the parallel state `P`:
    *   Exit actions for the active states within *all* regions are executed first (from deepest child upwards within each region).
    *   Then, the exit action of `P` itself (if any) is executed. The exact order of execution *between* orthogonal regions' exit actions is generally not guaranteed.
5.  **Event Processing**: When the state machine is in a parallel state `P` and receives an event:
    *   The event is dispatched to *all* active child regions concurrently.
    *   Each region attempts to handle the event based on its current state and transitions (including bubbling within that region).
    *   It's possible for multiple regions to react to the same event independently. All resulting transitions and actions within those regions will occur as part of processing the single incoming event.
    *   If the parallel state `P` *itself* defines a transition for the event, that transition takes priority over transitions defined within the regions (unless the event is handled entirely *within* a region without bubbling up to `P`). A transition defined on `P` will cause all regions to be exited.
6.  **Completion (Implicit)**: A parallel state implicitly reaches a "completed" status only when *all* of its orthogonal regions have independently reached a final state (Final states are TBD for v0.1, but this is the standard semantic). Transitions *out* of the parallel state can be conditioned on this completion, or triggered explicitly by events defined on the parallel state itself.

### 4.6. Delayed Transitions / Timers
_Transitions triggered by the passage of time. Defined using the `after DURATION => TARGET_STATE [action .optional_action];` syntax within a state body. When a state with an `after` transition is entered, an internal timer is started. If the state is exited before the timer fires, the timer is cancelled. If the timer fires, the specified transition occurs._

Delayed transitions allow a state machine to automatically transition to another state after a specified duration has elapsed, provided it remains in the source state for that duration. This is defined using the `after DURATION => TARGET_STATE [action .optional_action];` syntax within a state body.

1.  **Definition**: A delayed transition is specified within a state definition using the `after` keyword, followed by a duration (e.g., `500ms`, `2s`), the target state, and an optional transition action.
    *   Example: `state Waiting { after 5s => TimedOut [action .handle_timeout]; ... }`
2.  **Timer Activation**: When the state machine enters a state that defines one or more `after` transitions, internal timers corresponding to each defined delay are started.
3.  **Timer Cancellation**: If the state machine transitions *out* of the state *before* a delayed transition's timer fires, that specific timer is automatically cancelled. This ensures the delayed transition only occurs if the machine remains in the source state for the full duration.
4.  **Timer Firing**: If a timer associated with an `after` transition fires (i.e., the specified duration elapses while still in the source state), the state machine executes the corresponding transition:
    *   The source state is exited (triggering exit actions).
    *   The transition action (if specified in the `after` definition) is executed.
    *   The target state is entered (triggering entry actions).
5.  **Multiple Delays**: A state can define multiple `after` transitions with different durations and targets. Each will start its timer upon state entry. The first timer to fire will trigger its transition, cancelling any other pending timers defined *within the same source state*.
6.  **Interaction with Events**: Delayed transitions behave like internal events generated by the timer mechanism. If an external event triggers a transition *out* of the state before the timer fires, the external event takes precedence, and the timer is cancelled. If the timer fires, its transition is processed like any other event-triggered transition.
7.  **Implementation**: The underlying timer mechanism may depend on the enabled features.
    *   With `std` and `async` features, this might integrate with `tokio::time`.
    *   In `no_std` environments, a simpler tick-based approach or integration with platform-specific timers might be required. This typically involves the user providing timer services (perhaps via a `TickProvider` trait) or periodically calling a `tick()` method on the state machine or associated timer management struct. The precision and maximum duration may vary based on the implementation.

### 4.7. Invoked Services / Child Statecharts
_A state can invoke other services or child statecharts. This is defined using the `invoke child SERVICE_NAME -> statechart!(...);` syntax (actual invocation mechanism TBD). The parent statechart can send events to and receive events from the invoked child. The lifecycle of the child (start, stop) is typically tied to the parent state's entry and exit._

_(Phase 7 Target)_

Statecharts can invoke other long-running services or spawn child statecharts, managing their lifecycle and potentially communicating with them. This feature allows for composing complex systems from smaller, reusable state machine components.

1.  **Definition**: Invocation is declared within a state using the `invoke ...;` syntax. The exact syntax for specifying the invoked service/child and communication mapping is **To Be Defined** in Phase 7. A potential sketch is `invoke child ChildMachineName: ChildMachineType { /* optional event mapping */ };`.
2.  **Lifecycle**:
    *   **Activation**: When the parent state machine enters a state containing an `invoke` declaration, the specified child service or statechart instance is started/spawned.
    *   **Termination**: When the parent state machine exits the state containing the `invoke` declaration, the invoked child service or statechart instance is automatically stopped/terminated.
3.  **Communication (TBD)**: Mechanisms will be defined to allow:
    *   The parent machine to send events to the invoked child.
    *   The invoked child to send events back to the parent machine (potentially causing transitions in the parent).
    *   Sharing or mapping context data between parent and child.
4.  **Use Cases**: Useful for managing background tasks, interacting with external systems (represented as statecharts), or breaking down very large statecharts into more manageable, composable units.
5.  **Implementation Details**: The exact implementation will depend heavily on the `async` and `std` features, likely involving task spawning and message passing channels when available. `no_std` support might be limited or require specific external integration points.
6.  **Error Handling (Release Builds / `std` feature)**:
    *   When the `std` feature is enabled, operations that can potentially fail at runtime (e.g., interacting with invoked services, timer management if using fallible system calls) should ideally return `Result<T, E>` where appropriate. The exact error types are TBD. The core `send` method itself is designed to return `bool` (indicating if a transition occurred) and not `Result`, as failure to transition due to guards or lack of matching events is considered normal operation, not an error.
    *   Failures within invoked children might result in specific events being sent back to the parent machine.
7.  **`no_std` Environments**: In `no_std` environments without the `std` feature, the emphasis is heavily on compile-time validation. Runtime operations are designed to be infallible where possible. If unavoidable runtime failures can occur (e.g., timer allocation failure in a hypothetical `no_std` timer service), the behavior might involve specific error states, context flags, or defined fallback transitions rather than returning `Result`. Panics in release `no_std` builds **must** be avoided entirely; the generated code should strive to be compatible with `#![forbid(panic)]` in release mode.

### 4.8. History States (TBD)
_Shallow vs. deep history, default transitions._

_(TBD for v0.1)_

History states allow a state machine to remember and automatically re-enter the last active substate(s) of a compound or parallel state when it is transitioned back into.

1.  **Concept**: When transitioning out of a compound state that has a history mechanism, the machine recorded which substate(s) were active. If a later transition targets the compound state's history state marker, instead of entering the compound state's `initial:` substate, it directly enters the previously recorded substate(s).
2.  **Types**:
    *   **Shallow History**: Remembers and restores only the direct active child state of the compound state. If that child was itself compound, its *own* initial state is entered upon restoration.
    *   **Deep History**: Remembers and restores the full active state configuration *within* the compound state, down to the innermost nested atomic states.
3.  **Syntax**: Specific syntax (e.g., a `history` attribute or pseudo-state like `state H*`) is **To Be Defined**.
4.  **Use Cases**: Useful for implementing features like interruption and resumption, where returning to a parent state should resume the specific work-in-progress that was interrupted (e.g., restoring the specific tab or sub-menu a user was in).
5.  **v0.1 Status**: History states are **not targeted for v0.1**. This section serves as a placeholder for future specification if the feature is prioritized later.

---

## 5. Error Handling

### 5.1. Compile-Time Errors
_List of errors the macro should detect (e.g., unknown state, duplicate transition, unreachable region). Reference `statechart.mdc`._

The `statechart!` macro should perform extensive validation of the statechart definition at compile time, providing clear error messages to guide the user. Errors detected at compile time prevent the generation of incorrect or unsound state machine code.

Key compile-time errors include (but are not limited to):

1.  **Syntax Errors**:
    *   Malformed macro input that does not conform to the EBNF grammar (e.g., missing commas, incorrect keywords, unbalanced braces).
2.  **Header Field Errors**:
    *   Missing mandatory header fields (`name`, `initial`, `context`).
    *   Duplicate header fields.
    *   Invalid type for `context` (e.g., not a valid Rust type path).
    *   `initial:` state not defined in the statechart.
3.  **State Definition Errors**:
    *   Duplicate state names (at the same hierarchical level).
    *   `initial:` substate in a compound state not defined within that compound state.
    *   Missing `initial:` substate declaration in a compound state.
    *   Missing `initial:` substate declaration in any direct child region of a `[parallel]` state (though `initial` for the parallel state *itself* is not applicable).
    *   Invalid state attributes (e.g., `[foo]`). Unknown attribute on a state.
4.  **Transition Errors**:
    *   Transition target state not defined in the statechart.
    *   Event name in `on EVENT` not defined (if a global event enum is inferred or required, TBD). Currently, event names are identifiers.
    *   Duplicate identical transitions (same event, same source, same target, same guard if present) on the same state.
    *   Action method referenced in `[action .my_action]` not found on the `Context` struct, or has an incompatible signature.
    *   Guard method referenced in `[.guard .my_guard]` not found on the `Context` struct, or has an incompatible signature (e.g., does not return `bool`, takes `&mut Context`).
5.  **Hierarchy and Parallelism Errors**:
    *   Invalid `[parallel]` nesting (e.g., a direct child region of a `[parallel]` state cannot itself be `[parallel]` without an intermediate compound state; see rule in `statechart.mdc`).
    *   A parallel state must have at least two child regions.
6.  **Timer Errors**:
    *   Invalid `DURATION` format in `after DURATION ...`.
    *   Target state for an `after` transition not defined.
7.  **Unreachable States/Regions (Potentially)**:
    *   The macro *may* attempt to detect states or regions that can never be entered due to the transition logic. This can be complex and might be a best-effort feature or deferred.
8.  **Resolver Errors**:
    *   Failure to resolve Rust type paths or identifiers correctly.

The error messages should, where possible, point to the specific location in the macro input that caused the error.

### 5.2. Runtime Errors/Panics
_When (if ever) the runtime component might panic. Prefer `Result` types in `std` builds._

The generated state machine code aims to be robust and panic-free in release builds, especially for `no_std` targets.

1.  **Panics (Debug Builds Only)**:
    *   In debug builds (`debug_assertions` enabled), the runtime *may* panic under exceptional circumstances that indicate a fundamental logic error or violation of internal invariants (e.g., attempting to enter an invalid state representation, encountering corrupted internal data). These panics serve as early detection for bugs during development.
2.  **Panics (Discouraged in User Code)**: Panicking within user-provided action or guard methods is strongly discouraged as it can leave the state machine in an inconsistent state. Actions needing to signal failure should modify context or emit events instead.
3.  **Error Handling (Release Builds / `std` feature)**:
    *   When the `std` feature is enabled, operations that can potentially fail at runtime (e.g., interacting with invoked services, timer management if using fallible system calls) should ideally return `Result<T, E>` where appropriate. The exact error types are TBD. The core `send` method itself is designed to return `bool` (indicating if a transition occurred) and not `Result`, as failure to transition due to guards or lack of matching events is considered normal operation, not an error.
    *   Failures within invoked children might result in specific events being sent back to the parent machine.
4.  **`no_std` Environments**: In `no_std` environments without the `std` feature, the emphasis is heavily on compile-time validation. Runtime operations are designed to be infallible where possible. If unavoidable runtime failures can occur (e.g., timer allocation failure in a hypothetical `no_std` timer service), the behavior might involve specific error states, context flags, or defined fallback transitions rather than returning `Result`. Panics in release `no_std` builds **must** be avoided entirely; the generated code should strive to be compatible with `#![forbid(panic)]` in release mode.

---

## 6. Feature Flags

_As defined in `statechart.mdc` and `ROADMAP.md`. A potential future `trace` feature might be added for detailed instrumentation hooks, possibly integrating with the `tracing` crate._

### 6.1. `std`
_Enables Tokio mailbox, file I/O for diagrams, etc._

The `std` feature enables functionality that depends on the Rust standard library (`std`), including features requiring memory allocation (beyond potential stack usage) and integration with operating system services.

*   **Purpose**: To allow the statechart library to be used in environments where `std` is available (e.g., typical desktop/server applications) and leverage `std`-specific features. The library remains `#![no_std]` compatible by default when this feature is *not* enabled.
*   **Enabled Functionality**:
    *   Integration with `std::error::Error` trait (potentially via `thiserror`).
    *   Support for standard collections if needed internally (though `heapless` might still be preferred where applicable for performance/predictability).
    *   Use of `std::fmt` by default for generated enums (instead of `core::fmt`).
    *   Potential use of standard library primitives for timers or concurrency if the `async` feature is also enabled (e.g., `tokio` integration relies on `std`).
    *   File I/O capabilities, primarily used by the `diagram` feature for generating output files.
    *   Potentially richer debugging and logging integrations.
*   **Dependencies**: Enabling `std` pulls in optional dependencies like `anyhow`, `thiserror`, potentially parts of `tokio` (if `async` is also enabled), and `futures/std`. See `Cargo.toml`.

### 6.2. `async`
_Pulls `alloc`, `futures`, `async-trait`. Allows `async fn` in actions/guards._

The `async` feature enables integration with Rust's asynchronous programming ecosystem, allowing actions, guards, and potentially invoked services to perform non-blocking operations.

*   **Purpose**: To support use cases where state machine actions need to interact with external I/O or perform long-running computations without blocking the execution thread, particularly when used with the Actor Model (Section 7) in an async runtime like Tokio.
*   **Enabled Functionality**:
    *   Allows defining action and guard methods on the `Context` struct as `async fn`. The state machine runtime (especially the Actor Model) will correctly `await` these functions.
    *   Enables the Actor Model's `Mailbox::send(event).await` method for asynchronous event submission with back-pressure.
    *   Facilitates integration with async timer mechanisms (e.g., `tokio::time`) when the `std` feature is also enabled.
    *   Enables invoking child services/statecharts that operate asynchronously (details TBD in Phase 7).
*   **Dependencies**: Enabling `async` pulls in optional dependencies like `futures` (configured for `no_std` compatibility where possible) and `async-trait`. It implicitly requires an allocator (`alloc` crate), even in `no_std` environments, due to the nature of `async-trait` and `Future` pinning/state storage. If `std` is also enabled, `tokio` might be pulled in as well, depending on other features. See `Cargo.toml`.
*   **Core Semantics**: Even with `async` actions/guards, the state machine's core transition logic remains synchronous and sequential for a given event. The Actor Model ensures that an `async` action associated with a transition completes before the next event is processed from the mailbox, preserving determinism.

### 6.3. `diagram`
_Emits `TRANSITIONS` table, formatters for DOT/Mermaid. Off in firmware builds._

The `diagram` feature enables the generation of visual representations of the statechart definition, aiding in documentation and understanding.

*   **Purpose**: To provide tools for visualizing the structure and transitions of the state machine defined in the `statechart!` macro. This is primarily intended for documentation generation and debugging, not for runtime use in resource-constrained environments.
*   **Enabled Functionality**:
    *   Exposes internal data structures or metadata representing the statechart's topology (states, transitions, hierarchy, etc.). This might involve generating a constant data structure (e.g., `TRANSITIONS`) within the macro output, conditionally compiled based on this feature.
    *   Provides functions or methods (e.g., `MyStateMachine::to_dot()` or similar) to format this structural information into common diagram description languages.
    *   **Supported Formats (Target)**: Graphviz DOT language (`.dot`) and Mermaid flowchart syntax (`.mmd`).
    *   These formatters allow rendering the statechart using external tools (like Graphviz) or directly in Markdown environments that support Mermaid (like GitHub).
*   **Dependencies**: Enabling `diagram` pulls in optional dependencies like `serde` (for serializing the internal representation if needed by the formatters). If file output helpers are provided, it might also implicitly require the `std` feature for file I/O.
*   **`no_std` Impact**: This feature is generally *not* intended for use in `no_std` firmware builds due to its purpose (offline generation) and potential dependencies (`serde`, `alloc`, possibly `std`). The generated metadata structure itself might be `no_std` compatible, but the formatting functions likely require `alloc` or `std`. Build configurations for firmware should typically disable this feature.

---

## 7. Actor Model (Phase 4 Target)

_(Phase 4 Target)_

To facilitate integration into concurrent applications, especially when using the `std` or `async` features, `lit-bit` provides an actor model layer. This wraps the core state machine logic, providing a message-passing interface.

1.  **Core Traits (Conceptual - Specific names TBD)**:
    *   `Actor`: Represents the running state machine instance within the actor system. It encapsulates the state machine, its context, and a mailbox.
    *   `Mailbox`: An interface for sending events asynchronously to the actor's internal queue. Implementations will vary based on features (`heapless::spsc::Queue<Event, const N: usize>` for `no_std` with generic capacity `N`, potentially `tokio::sync::mpsc::channel` when used with an `std` async runtime like Tokio).
2.  **Event Processing Loop**:
    *   The `Actor` runs an internal processing loop (potentially on a spawned task if `async` is enabled).
    *   This loop dequeues events one at a time from the `Mailbox`.
    *   Each dequeued event is processed by calling the core state machine's `send()` method.
    *   **Single-threaded Guarantee**: Crucially, the actor ensures that `send()` is called sequentially for each event. Even if actions involve `async` operations, the actor `await`s their completion *before* processing the next event in the queue, maintaining the state machine's synchronous execution semantics internally.
3.  **Interface**:
    *   Instead of calling `send()` directly, users interact with the `Actor` via its `Mailbox`.
    *   `try_send(event)`: Attempts to queue an event immediately, returning `Err(event)` if the mailbox is full (providing back-pressure). This is suitable for synchronous or `no_std` contexts.
    *   `send(event).await`: (Requires `async` feature) Asynchronously sends an event, potentially waiting if the mailbox is full until space becomes available.
4.  **`no_std` Considerations**:
    *   The actor model in `no_std` environments will rely on `heapless` queues with fixed capacities, making `try_send` the primary interaction method.
    *   No dynamic memory allocation (global alloc) will be used in the `no_std` actor implementation.
5.  **Instrumentation**:
    *   (Requires `trace` feature, TBD) The actor layer may provide hooks or emit trace events (e.g., using the `tracing` crate) for state transitions (`on_transition(old_state, event, new_state)`), mailbox status, and event processing, allowing users to observe the actor's behavior.

This optional layer provides a standardized way to integrate the state machine into larger concurrent systems while preserving its core execution guarantees. The specific implementation details are targeted for Phase 4.

---

## 8. Future Considerations (Post v0.1)

_Ideas for v0.2 and beyond (e.g., statechart inspection API, advanced testing utilities, SCXML import/export if demand exists)._

*   History States (Shallow & Deep)
*   Parallel JOIN Transitions (Completion of all nested states in parallel regions)
*   Statechart Inspection/Serialization API
*   Event Payloads (Allowing events to carry data)
*   More sophisticated Timer options (e.g., cron-like scheduling)
*   SCXML Import/Export

---

## 9. Design Insights & Mitigation Strategies (2025-05 Research Audit)

The following distilled lessons are drawn from an audit of Rust-native state-machine crates (e.g. `statig`, `rust-fsm`, `async_fsm`) and from prior art such as Boost.SML (C++) and XState (JavaScript).  They inform lit-bit's public API, CI policy, and roadmap.

1. **Context Lifetimes & Ownership**
   * Use flexible lifetime parameters or GATs so context borrowing does not over-constrain the API (addresses `statig` #19).
   * Prefer *owned* event payloads to sidestep complex lifetime chains; allow borrowing as an optimisation, not a requirement.
2. **Compile-Time & Binary-Size Budget**
   * Macro expansion must scale *linearly* with the number of states/events.  CI runs a `bench_1000_states` crate and fails if compilation exceeds 30 s or binary size regresses >10 %.
   * Expensive tooling (diagram export, tracing) is **feature-gated** so typical firmware builds stay lean.
3. **Hierarchy Semantics**
   * Every compound state **must** declare an `initial:` sub-state; the macro emits a compile-time error otherwise.
   * Parent→child and child→parent transitions execute entry/exit actions exactly once; tests assert correct LCA behaviour.
4. **Async & Timer Determinism**
   * The Actor layer serialises event handling; a transition (incl. awaited actions) must finish **before** the next event dequeues.
   * Timers are cancelled automatically on state exit via an internal `TimerHandle` abstraction.
5. **Diagram Generation Accuracy**
   * The `diagram` feature generates DOT/Mermaid directly from the transition table during compilation, eliminating stale docs.
   * A CI check parses the emitted graph to ensure every defined state/transition appears exactly once.
6. **Soundness & `unsafe` Policy**
   * Core crates carry `#![deny(unsafe_code)]`.  Any unavoidable `unsafe` is isolated behind a feature flag (`unsafe_opt`) and documented.
   * Fuzz and MIRI jobs run nightly to detect UB or double-drop scenarios across random event sequences.
7. **Custom Lints & Clippy Pedantic**
   * Development builds enable `clippy::pedantic`; additional lints flag duplicate state attributes, large enum variants, or reference-holding state structs.

> These mitigations feed directly into the updated roadmap KPIs and CI steps (see `ROADMAP.md`).
