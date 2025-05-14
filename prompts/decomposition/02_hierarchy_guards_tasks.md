# Phase 2 · Hierarchy & Guards — Task Decomposition

This document breaks down the deliverables from `prompts/phases/02-hierarchy-guards/02_hierarchy_guards_checklist.md` into actionable sub-tasks.

## 1. Implement Parent/Child State Concepts

*   **Goal:** Allow states to be nested within other states.
*   **Tasks:**
    *   [ ] **Design Data Structures:**
        *   [ ] Modify `MachineDefinition` (or a new structure) to represent state hierarchy (e.g., a tree or nested enum structure).
        *   [ ] Consider how `StateId` (if still used) will represent nested states (e.g., path-like identifiers, or tuple of parent/child IDs).
    *   [ ] **Update Runtime Logic:**
        *   [ ] Modify `Runtime::current_state` to accurately reflect the active child state within its parent(s).
        *   [ ] Adapt transition matching logic to consider the current state's position in the hierarchy (e.g., events might be handled by parent states if not by the child).
    *   [ ] **Macro Changes (`statechart!`):**
        *   [ ] Design syntax for defining nested states within the macro (e.g., nested `state {}` blocks).
        *   [ ] Update macro to parse this new syntax and generate the hierarchical state representation.

## 2. Ensure Correct Entry/Exit Action Order for Nested States

*   **Goal:** Actions must fire in the correct order when transitioning between nested states.
*   **Tasks:**
    *   [ ] **Implement Exit Action Logic:**
        *   [ ] When transitioning out of a nested state, ensure exit actions are called from the most specific active child state up to the parent state that is being exited.
        *   [ ] This might involve traversing the state hierarchy upwards from the current leaf state.
    *   [ ] **Implement Entry Action Logic:**
        *   [ ] When transitioning into a nested state, ensure entry actions are called from the least specific parent state being entered down to the target child state.
        *   [ ] This might involve traversing the state hierarchy downwards to the target leaf state.
    *   [ ] **Refine `Runtime::send`:**
        *   [ ] Integrate the new entry/exit action logic into the transition execution part of `Runtime::send`.

## 3. Unit Tests for Parent/Child State Transitions

*   **Goal:** Verify hierarchy and entry/exit action order.
*   **Tasks:**
    *   [ ] **Basic Nesting:**
        *   [ ] Test transition from `ParentA::Child1` to `ParentA::Child2` (entry/exit within same parent).
        *   [ ] Test transition from `ParentA::Child1` to `ParentB::Child1` (entry/exit across different parents).
    *   [ ] **Deep Nesting (e.g., 3 levels):**
        *   [ ] Test transitions between deeply nested children, ensuring all intermediate parent entry/exit actions fire correctly.
    *   [ ] **Transition to Parent State:**
        *   [ ] If transitioning to a parent state that has a default initial child, ensure the child's entry actions also fire.
    *   [ ] **Test Action Order Explicitly:**
        *   [ ] Use mock actions or loggers to verify the sequence of action calls.

## 4. Implement Guard Conditions on Transitions

*   **Goal:** Allow transitions to be conditional based on context or event data.
*   **Tasks:**
    *   [ ] **Update `Transition` Struct:**
        *   [ ] Add an optional field for a guard function (e.g., `Option<fn(&C, &E) -> bool>` or similar, considering `dyn Fn`).
    *   [ ] **Modify `Runtime::send`:**
        *   [ ] Before executing a transition, if a guard is present, call it.
        *   [ ] Only proceed with the transition if the guard returns `true`.
    *   [ ] **Event Matching with Guards:**
        *   [ ] Clarify behavior if multiple transitions match an event: typically, the first defined transition whose guard passes is taken.
    *   [ ] **Macro Changes (`statechart!`):**
        *   [ ] Design syntax for specifying guards in the macro (e.g., `on Event [guard path::to::guard_fn] => NextState`).
        *   [ ] Update macro to parse guard syntax and populate the `Transition` struct.

## 5. Unit Tests for Guard Conditions

*   **Goal:** Verify guard logic.
*   **Tasks:**
    *   [ ] Test transition occurs when guard returns `true`.
    *   [ ] Test transition is prevented when guard returns `false`.
    *   [ ] Test that context and event data are correctly passed to the guard function.
    *   [ ] Test scenarios with multiple potential transitions for the same event, where guards determine which is taken.

## 6. Compile-Time Error for Unknown State in Transition

*   **Goal:** Prevent typos and logic errors by ensuring all states referenced in transitions are actually defined.
*   **Tasks:**
    *   [ ] **Design Macro Logic:**
        *   [ ] During macro expansion, collect all defined state names.
        *   [ ] For each transition, verify that `from_state`, `to_state` (and any other state references) exist in the collected set.
        *   [ ] If an unknown state is found, emit a `compile_error!`.
    *   [ ] **Error Reporting:**
        *   [ ] Ensure the `compile_error!` message is clear and indicates which state name was problematic.

## 7. Unit Test for Unknown State Compile-Time Error

*   **Goal:** Confirm the compile-time error mechanism works.
*   **Tasks:**
    *   [ ] Create a `tests/compile-fail/` directory if not already present.
    *   [ ] Add a test case (e.g., `unknown_state_transition.rs`) that deliberately uses the `statechart!` macro to define a transition to a non-existent state.
    *   [ ] Use `trybuild` or similar crate (as a dev-dependency) to assert that this test case fails to compile with the expected error message.
    *   *(Alternative if compile-time is too hard for this specific check initially: Implement a robust runtime check during `MachineDefinition` construction that panics or returns `Result`, and test that.)*

## 8. Review and Update Core Runtime

*   **Goal:** Ensure existing core logic is compatible with new features.
*   **Tasks:**
    *   [ ] After implementing hierarchy and guards, review `src/core/mod.rs` for any required refactoring or adjustments.
    *   [ ] Ensure `StateMachine` trait remains appropriate or update as needed.

## 9. No New Heap Allocations & Performance

*   **Goal:** Maintain `#![no_std]` compatibility and performance.
*   **Tasks:**
    *   [ ] Continuously monitor for accidental heap allocations, especially in the `statechart!` macro expansion or new runtime logic.
    *   [ ] Use tools like `cargo-bloat` or manual inspection if concerned.

## 10. Clippy Pedantic

*   **Goal:** Maintain code quality.
*   **Tasks:**
    *   [ ] Regularly run `cargo clippy --all-targets -- -D warnings -A clippy::type_complexity` (or similar, matching project's clippy setup).
    *   [ ] Address any new `pedantic` lints introduced by Phase 2 code. 