# Project Progress Journal

Reverse-chronological log of daily coding sessions.  Keep entries **concise** and link to PRs / issues for full detail.

_Add new sessions below this line._ 

## 2024-12-28 · _Session End_ (Major Code Review Issues Completed) ✅
* _Author_: @AI-agent (via @0xjcf)
* _Phase_: 1-core-runtime (Code Review Response & Refactoring)
* _Work_: **Major Refactoring Completed**: (1) Fixed unwrap_or(false) calls to properly propagate ProcessingError instead of silently converting to false, (2) Fixed Scratch struct buffer size from M to MAX_NODES_FOR_COMPUTATION for parallel state compatibility, (3) **Completed major send_internal refactoring** - extracted complex 2,500+ line method into modular helper functions (collect_potential_transitions, arbitrate_transitions, apply_exits_and_run_actions, apply_entries, commit_leaves), (4) Replaced duplicated transition collection logic with calls to existing helper methods, (5) Fixed Copy derive issue in Spec.md MediaPlayerEvent enum, (6) Fixed Cargo.toml empty workspace.package section, (7) Fixed unused variable in lit-bit-macro, (8) Fixed format string issues in parallel test assertions. **All 14 core tests + 53 macro tests still passing after major refactoring.** Only minor clippy issues remain (type complexity, line count warnings).
* _Next_: Address remaining minor linter issues or proceed with next phase development.

## 2024-12-28 · _Session End_ (Uninlined Format Args Fixed) ✅
* _Author_: @AI-agent (via @0xjcf)
* _Phase_: 1-core-runtime (Code Quality & Linting)
* _Work_: Fixed clippy::uninlined_format_args warnings in parallel_machine_integration_test.rs by inlining format arguments in assert! and assert_eq! macros. Changed format strings like `"Log: {:?}", variable` to `"Log: {variable:?}"` for cleaner, more modern Rust code. **All 14 core tests + 53 macro tests passing, linter clean with exit code 0.**
* _Next_: Address any remaining code review feedback or move to next development milestone.

## 2024-12-28 · _Session End_ (Linter Issues Fixed) ✅
* _Author_: @AI-agent (via @0xjcf)
* _Phase_: 1-core-runtime (Code Quality & Linting)
* _Work_: Fixed all remaining linter errors and warnings: (1) Collapsed nested if statements in send_internal method as suggested by clippy, (2) Added #[allow(dead_code)] annotations to unused helper methods (collect_potential_transitions, arbitrate_transitions) that were created for future refactoring, (3) Added #[allow(dead_code)] annotations to all test helper types, functions, and constants in the tests module to suppress dead code warnings, (4) Fixed redundant closure and uninlined format args warnings in basic_machine_integration_test.rs, (5) Cleaned up warnings in external_events.rs example by adding #[allow(unused_imports)] for StateMachine trait import and #[allow(dead_code)] for demonstration function. **All 14 core tests + 53 macro tests passing, linter completely clean with exit code 0 across entire workspace including examples.**
* _Next_: Continue with final minor code review issues or begin next development phase.

## 2024-12-28 · _Session End_ (Code Review Issues Largely Complete) ✅
* _Author_: @AI-agent (via @0xjcf)
* _Phase_: 1-core-runtime (Code Review & Quality Improvements - Round 3)
* _Work_: Completed majority of latest code review fixes: (1) Fixed bracket_token.span.join() call, (2) Updated Tokio dependency from 1.41 to 1.42, (3) Added StateNode instances to LIGHT_STATENODES array, (4) Added Debug trait to StateNode (was already present), (5) Fixed matches! macro type mismatch with dereferencing, (6) Added Copy derive to MediaPlayerEvent in Spec.md, (7) Gated println! statements with #[cfg(feature = "std")] for no_std compatibility, (8) Replaced panic! with ProcessingError::CapacityExceeded returns, (9) Fixed debug-log feature to include dep:log, (10) Added helper functions for send_internal refactoring (collect_potential_transitions, arbitrate_transitions), (11) Fixed SendResult::Error handling in examples by adding proper match statements. All tests now compile and pass except for one minor duplicate function warning. Core functionality fully working.
* _Next_: Complete send_internal refactoring by integrating helper functions, finalize remaining minor issues.

## 2024-12-28 · _Session End_ (Latest Code Review Fixes Complete) ✅
* _Author_: @AI-agent (via @0xjcf)
* _Phase_: 1-core-runtime (Code Review & Quality Improvements)
* _Work_: Completed comprehensive code review addressing 9 identified issues: (1) ~~Reverted M value calculation to use all_states.len() instead of max_depth() - tests showed hierarchy depth wasn't sufficient for internal operations~~, (2) Fixed Transition struct initialization to use match_fn field instead of outdated event field, generating proper matcher functions, (3) Updated match arm patterns to correctly match references by prefixing event patterns with &, (4) Fixed dangling references in Scratch struct creation by using named mutable Vec variables, (5) Fixed shared entry_actions_run_vec across all Scratch instances in send_internal to prevent repeated execution, (6) Updated external_events.rs example to properly capture and handle SendResult return values, (7) Fixed tests to directly compare StateId enum variants instead of string conversion, (8) Resolved all clippy warnings including unused parameter prefixes and format args issues, (9) Removed unused max_depth() method causing warnings. **All 28 core tests + 53 macro tests passing, linter clean, no warnings or errors.**
* _Next_: Continue with Phase 1 deliverables, consider moving to Phase 2 (Hierarchy & Guards) implementation and testing.

## 2025-05-23 · _Session End_ (Embedded Build Fixes) ✅
* _Author_: @AI-agent (via @0xjcf)
* _Phase_: 1-core-runtime (Build fixes for embedded targets)
* _Work_: Fixed `just run-rv` and `just size-check-cortex-m` commands. (1) Removed unnecessary `lit-bit-macro-support` crate and all dependencies on it, (2) Cleaned up `#[statechart_event]` macro to remove unused EventMeta generation, (3) Updated traffic_light examples for both RISC-V and Cortex-M to use new API (match_fn, proper imports, Runtime::new signature), (4) All builds passing, RISC-V example runs in QEMU successfully, Cortex-M size check reports ~10KB text section. **(5) Fixed macro imports by adding lit-bit-macro as dependency and re-exporting from lit-bit-core. All tests passing, linter clean.**
* _Next_: Continue with Phase 1 deliverables, consider adding no_std tests in CI to prevent regressions.

## 2025-05-23 · _Session End_ (Pattern Matching Refactor) ✅
* _Author_: @AI-agent (via @0xjcf)
* _Phase_: 1-core-runtime / macro (Event Pattern Matching Refactor)
* _Work_: Successfully refactored event pattern matching implementation. (1) Updated Spec.md to document `#[statechart_event]` requirement with rationale, (2) Created external_events.rs example showing three patterns for handling external enums, (3) Moved `#[statechart_event]` proc macro before tests module to fix linter, (4) Fixed test expectations for new Transition format without event field, (5) All tests passing, examples compile and run correctly. **(6) Added match_fn check in Runtime::send_internal to properly match events during transitions.** **(7) All linter issues resolved, full test suite passing.**
* _Next_: Consider adding more examples for complex event patterns, test const-correctness with no_std builds, and potentially optimize generated match functions.

## 2025-05-23 · _Session Start_
* _Author_: @claude-4-opus (via @0xjcf)
* _Phase_: 1-core-runtime / macro (Event Pattern Matching Refactor)
* _Work_: Starting refactor of event pattern matching implementation based on research findings. The research confirmed that requiring `#[statechart_event]` is the industry-standard approach (similar to Strum, Serde, enum_dispatch). Will implement discriminant enum approach to fix const context issues.
* _Next_: (1) Update Spec.md documentation about `#[statechart_event]` rationale, (2) Create examples for external event enum handling, (3) Refactor macro to use discriminant enums instead of dummy values.

## 2025-05-22 · _Session Start_
* _Author_: @AI-agent Default (via @0xjcf)
* _Phase_: 1-core-runtime
* _Work_: Fixed hierarchical state entry/exit bugs: (1) cross-root transition fallback/entry path, (2) double entry on child-to-parent, (3) dedup for parallel region leaf self-transition. All changes respect linter and explicit naming rules. See commit for details.
* _Next_: Re-run full test suite and verify all hierarchical/parallel transition tests pass. 

## 2024-05-21 · _Session End_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Macro Refactor & Test Corrections)
*   _Work_:
    *   Refactored the `statechart!` macro's code generation for the `send` method. It now delegates directly to `Runtime::send_internal` in `lit-bit-core`. This resolved a persistent E0317 linter error ("if may be missing else clause") and fixed numerous test failures related to incorrect transition processing in hierarchical and parallel states.
    *   Removed the `generate_send_method` and `analyse_event_pattern` functions from `lit-bit-macro` as they are no longer needed.
    *   Addressed various Clippy lints (`dead_code`, `trivially_copy_pass_by_ref`, `uninlined_format_args`, `unreachable_code`) across `lit-bit-macro` and `lit-bit-core`.
    *   Corrected test logic in `core::tests::send_event_no_transition_if_guard_fails` and standardized log messages in `basic_machine_integration_test::parallel_initial_state_test`.
    *   Temporarily commented out the `LoadTrack` event and associated logic in the `media_player.rs` example to resolve compile errors from its incomplete implementation, enabling other tests to pass.
*   _Next_:
    *   Re-evaluate the implementation of events with data (like `LoadTrack`) and guards, ensuring the new `Runtime::send_internal` delegation handles them correctly.
    *   Address any remaining failing tests or new issues arising from this significant refactor.
    *   Continue with Phase 03 tasks, focusing on robust parallel state functionality and the media player example.


## 2024-05-19 · _Session Start_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Enhancement: Macro Event Pattern Matching)
*   _Work_:
    *   Pivoting to implement pattern matching for events with associated data directly within the `statechart!` macro (e.g., `on Event::Variant { field } => ...`).
    *   Modified `TransitionDefinitionAst` and `Parse` impl to use `event_pattern: syn::Pat` (via `Pat::parse_single`).
    *   Updated `TmpTransition` to hold `event_pattern: &'ast syn::Pat`.
    *   Updated `code_generator::generate_machine_struct_and_impl` to generate a `send()` method with `match event { ... }` structure using these patterns. Placeholder for full transition logic.
    *   Temporarily used `compile_error!` for `Transition.event` field in `generate_transitions_array` to handle type mismatch during refactor.
*   _Next_:
    *   Fix linter errors in `lit-bit-macro` tests related to `event_name` vs `event_pattern`.
    *   Update `GuardFn` type in `lit-bit-core` to take `&EventType`.
    *   Update macro code generation for guards to pass `&event`.
    *   Implement and test the actual transition logic (LCA, entry/exit actions) within the generated `send()` method.
    *   Verify full functionality with `media_player.rs` example, including events with data and guards.

---

## 2024-05-19 · _Session End_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Runtime Bugfixing & Test Completion)
*   _Work_:
    *   Successfully debugged and fixed all remaining failing unit and integration tests for hierarchical and parallel state transitions.
    *   Key fixes involved multiple iterations on:
        *   `find_lca` to correctly determine the lowest common ancestor.
        *   `execute_entry_actions_from_lca` and `enter_state_recursive_logic` to ensure correct entry action sequencing, avoid duplicates, and properly handle region/child initialization (using `run_entry_action_for_this_state` flag and explicit entry for region containers).
        *   De-duplication logic in `Runtime::send()` for `arbitrated_transitions` to prevent multiple executions of the same transition definition.
    *   Refined test assertions for parallel state logs to be robust against valid interleaving of actions from orthogonal regions.
    *   Addressed numerous Clippy lints in both `lit-bit-core/src/core/mod.rs` (tests module) and `lit-bit-core/tests/parallel_machine_integration_test.rs`.
    *   Added several new integration tests for parallel state scenarios in `parallel_machine_integration_test.rs`.
    *   All 28 `lit-bit-core` tests and 52 `lit-bit-macro` tests are now passing.
*   _Next_:
    *   Complete the final integration test for parallel states (event targeting a single region).
    *   Proceed with Task 4.3 (media player example) and Task 5 (documentation).

---

## 2024-05-19 · _Session Start_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Runtime Implementation)
*   _Work_: Began session to implement parallel state event dispatch in `Runtime::send()`.
*   _Next_: Focus on Task 3.2: Completing `Runtime::send()` for parallel states, then Task 4.1: Unit tests for this logic.

---

## 2025-05-18 · _Session End (Code Review Follow-up Part 2)_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Runtime Refinement & Polish)
*   _Work_:
    *   Addressed a second batch of code review suggestions following initial lint/test fixes:
    *   Core Runtime (`lit-bit-core/src/core/mod.rs`):
        *   Changed `Runtime::new` to accept `&'static MachineDefinition` (API change).
        *   Updated `compute_ordered_exit_set` to return `Result` and `send()` to abort on error.
        *   Updated `find_lca` to return `Result` and `send()` to abort on error.
        *   Ensured `visited_during_entry` in `enter_state_recursive_logic` uses capacity `M`.
        *   Updated `is_descendant_or_self` to return `Result` and updated callers in `send()`.
        *   Modified `enter_state_recursive_logic` to return `Result<(), EntryError>` and updated callers to handle/panic.
        *   Ensured `potential_transitions.push()` errors in `send()` lead to `return false`.
        *   Implemented `Display` and `std::error::Error` for `PathTooLongError` and `EntryError`.
        *   Added doc comment to `get_active_child_of`.
    *   Macro (`lit-bit-macro/src/lib.rs`):
        *   Updated macro to pass `&MACHINE_DEF_CONST` to `Runtime::new`.
        *   Refactored loop for finding colliding variant names in `generate_state_id_logic`.
        *   Changed `max_nodes_for_computation_val` to use `quote!{ M_VAL * lit_bit_core::core::MAX_ACTIVE_REGIONS }` and ensured it's used as `{ expr }` in generic arguments.
    *   Tests (`lit-bit-core/tests/basic_machine_integration_test.rs`):
        *   Updated tests to use `static MachineDefinition`s for `&'static` lifetime compliance.
        *   Increased `ACTION_LOG_CAPACITY` and implemented `hstr!` macro for test ergonomics.
    *   Examples (`lit-bit-core/examples/`):
        *   Updated examples (`traffic_light.rs`, `traffic_light_cortex_m.rs`) for `Runtime::new` taking `&'static MachineDefinition`.
        *   Re-exported `MAX_ACTIVE_REGIONS` from `lit-bit-core` crate root.
        *   Added type alias for `Runtime` in `traffic_light.rs`.
        *   Renamed/optimized `M` and related consts in `traffic_light_cortex_m.rs` and updated comments.
    *   The typo in the previous PROGRESS.md entry was fixed implicitly by creating this new entry structure.
    *   Addressed an unclosed delimiter error in `core/mod.rs` test section caused by previous model edits.
*   _Next_:
    *   Submit current changes for a new code review.
    *   Focus on critical deferred items: 
        *   `retain` predicate logic in `send()` for parallel composite self-transitions.
        *   Optimization of child lookup for parallel states (macro & core).
        *   Optimization of the arbitration loop in `send()`.
    *   Consider other deferred items like further error propagation (e.g. from `execute_entry_actions_from_lca`).

---

## 2025-05-17 · _Session End (Linter & Runtime Refinements)_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Runtime Refinement)
*   _Work_:
    *   Fixed a runtime bug where simple leaf self-transitions would result in an empty active state set by ensuring the state is correctly processed through the entry execution path in `Runtime::send()`.
    *   Addressed a comprehensive set of Clippy linter warnings and compilation errors across `lit-bit-core`, `lit-bit-macro`, and example files. This included fixes for `unreachable_code`, `manual_assert`, `uninlined_format_args`, const generic argument inference in examples, and `const fn` compatibility.
    *   Corrected state definitions in the `traffic_light.rs` (RISC-V) example to resolve a runtime hang.
    *   Refactored test assertions in `basic_machine_integration_test.rs` for clarity and correctness.
*   _Next_:
    *   (This was the start of the session that just ended, the work above effectively replaces these next steps as they were completed or superseded by the new review).

---

## 2025-05-17 · _Session End (Parallel Runtime Logic - Exit Implemented)_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Runtime Implementation)
*   _Work_:
    *   Refactored `Runtime::new()` in `lit-bit-core` to correctly initialize `active_leaf_states` when the machine's overall initial state is a parallel state. (Task 3.5 DONE from `prompts/decomposition/03_parallel_states_tasks.md`).
    *   Refactored state entry helpers (`enter_state_recursive_logic`, `execute_entry_actions_from_lca`) in `lit-bit-core` to support parallel semantics and return `heapless::Vec` of active states. (Task 3.3 DONE from `prompts/decomposition/03_parallel_states_tasks.md`).
    *   Implemented new hierarchical and parallel-aware exit logic:
        *   Added `clear_and_exit_state` helper for post-order exit of a state and its active children/regions.
        *   Updated `Runtime::send()` to use this helper in an upward traversal from the exited leaf to LCA, replacing the old exit logic. (Task 3.4 DONE from `prompts/decomposition/03_parallel_states_tasks.md`).
    *   Corrected self-transition logic in `Runtime::send()` to ensure proper exit/entry actions.
    *   Resolved all linter errors and fixed all failing tests in `lit-bit-core` and `lit-bit-macro`. All tests now passing.
    *   Added new integration test `test_initial_parallel_state_activation`.
*   _Next_:
    *   Complete the refactor of `Runtime::send()` (Task 3.2):
        *   Implement full arbitration logic for `potential_transitions` to correctly select transitions when multiple are found (e.g., parent vs. child, multiple regions).
        *   Modify the execution phase of `send()` to handle multiple, independent, arbitrated transitions that might occur in parallel regions from a single event. This includes correctly updating `active_leaf_states`.
    *   Add comprehensive unit and integration tests for parallel state transitions, event dispatch, and exit/entry action order (Tasks 4.1, 4.2 from `prompts/decomposition/03_parallel_states_tasks.md`).

---

## 2025-05-17 · _Session Mid (Runtime::new Parallel Init & Linter/Test Fixes)_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Runtime Implementation)
*   _Work_:
    *   Refactored `Runtime::new()` in `lit-bit-core` to correctly initialize `active_leaf_states` when the machine's overall initial state is a parallel state. (Task 3.5 DONE from `prompts/decomposition/03_parallel_states_tasks.md`).
    *   Refactored state entry helpers (`enter_state_recursive_logic`, `execute_entry_actions_from_lca`) in `lit-bit-core` to support parallel semantics and return `heapless::Vec` of active states. (Task 3.3 DONE from `prompts/decomposition/03_parallel_states_tasks.md`).
    *   Resolved all outstanding linter errors and test failures in `lit-bit-core` and `lit-bit-macro` related to these changes and previous refactors.
    *   Added new integration test `test_initial_parallel_state_activation` which is passing.
*   _Next_:
    *   Begin major refactor of `Runtime::send()` to handle event dispatch to/from parallel regions and manage transitions involving parallel states (Task 3.2).
    *   Develop parallel-aware exit logic, likely by refactoring `execute_exit_actions_up_to_lca` or creating `exit_state_recursive_logic` (Task 3.4).

---

## 2025-05-17 · _Session Start_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Runtime Implementation)
*   _Work_:
    *   Created new branch `feat/parallel-states` and committed foundational P0 and P1 (data structures) work for parallel states.
*   _Next_:
    *   Begin refactoring `Runtime::send()` in `lit-bit-core/src/core/mod.rs` to correctly handle event dispatch to multiple active regions in parallel states (Task 3.2).
    *   Concurrently, develop and refine the necessary state entry/exit logic (`execute_entry_actions_from_lca`, `execute_exit_actions_up_to_lca`, `enter_submachine_to_initial_leaf`) to support parallel semantics (Tasks 3.3, 3.4).
    *   Address updates to `Runtime::new()` for initial activation of parallel states as needed (Task 3.5).

---

## 2025-05-17 · _Session End (Planned Next: Parallel Runtime Logic)_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (P0 Tasks Complete, P1 Structs Done)
*   _Work_:
    *   Fixed all linter errors in `lit-bit-macro` and `lit-bit-core`.
    *   Completed all P0 tasks for Parallel States (Macro Syntax, Parsing, Semantic Analysis & Validation, including tests and `Spec.md` updates). (Tasks 1.1-1.4, 2.1-2.3 from `prompts/decomposition/03_parallel_states_tasks.md`)
    *   Updated `StateNode` in `lit-bit-core` to include `is_parallel: bool`.
    *   Updated `Runtime` in `lit-bit-core` to use `active_leaf_states: heapless::Vec` for managing multiple active states.
    *   Updated `StateMachine` trait (in `lit-bit-core/src/lib.rs`) and macro-generated code for `fn state()` to return `heapless::Vec`.
    *   Updated integration tests to correctly assert `machine.state().as_slice()`.
*   _Next_:
    *   Begin core runtime implementation for parallel states in `lit-bit-core/src/core/mod.rs`:
        *   Refactor `Runtime::send()` to handle event dispatch to multiple active regions in a parallel state (Task 3.2).
        *   Implement correct entry/exit logic for parallel states and their regions (Tasks 3.3, 3.4).
        *   Update `Runtime::new()` to handle initial activation of parallel states (Task 3.5).

---

## 2025-05-17 · _Session Start_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 03-parallel-states (Begin Implementation)
*   _Work_:
    *   Completed re-entry process, initiating Phase 03 planning.
    *   Created `prompts/phases/03-parallel-states/03_checklist.md`.
    *   Created `prompts/decomposition/03_parallel_states_tasks.md`.
    *   Confirmed alignment of Phase 03 plan with `Spec.md` and `ROADMAP.md`, clarifying that the `[parallel]` attribute syntax from `Spec.md` is to be used for parallel states.
*   _Next_:
    *   Confirm and document usage of the `[parallel]` attribute for parallel states as defined in `Spec.md` (Task 1.1 from `03_parallel_states_tasks.md`).
    *   Update `StateAttributes` in `lit-bit-macro/src/parser/ast.rs` to include a representation for the `[parallel]` attribute (Task 1.2 from `03_parallel_states_tasks.md`).

---

## 2025-05-16 · _Session End_
*   _Author_: @0xjcf (with @Gemini)
*   _Phase_: 02-hierarchy-guards (Phase Complete!)
*   _Work_:
    *   Verified and completed explicit action order testing for hierarchical transitions using `HierarchicalActionLogContext`.
    *   Implemented and tested scenarios for multiple guard selection, ensuring correct transition arbitration.
    *   Confirmed compile-time error reporting for unknown transition target states is working as expected.
    *   Added `trybuild` dev-dependency and compile-fail test case (`unknown_target_state.rs`) for unknown state validation.
    *   Moved `wip/unknown_target_state.stderr` to `lit-bit-macro/tests/compile-fail/` to finalize the compile-fail test.
    *   Updated all Phase 02 checklists in `prompts/phases/02-hierarchy-guards/02_checklist.md` and `prompts/decomposition/02_hierarchy_guards_tasks.md` to reflect completion.
*   _Next_:
    *   Review and commit all Phase 02 changes.
    *   Begin planning for Phase 03 (Parallel States).

---

## 2025-05-16 · _Session Start_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 02-hierarchy-guards (Testing Refinement)
*   _Work_:
    *   Reviewed re-entry prompt and prior `PROGRESS.md`.
    *   Conducted code review (grep) and discovered guard condition functionality (macro parsing, runtime logic, basic tests) was already substantially implemented.
    *   Updated `prompts/phases/02-hierarchy-guards/02_checklist.md` and `prompts/decomposition/02_hierarchy_guards_tasks.md` to reflect completed guard work.
    *   Analyzed `cargo-llvm-cov` report, identifying good overall coverage but potential gaps in explicit action order testing for hierarchy and multi-guard scenarios.
    *   Formulated a refined testing plan to address these specific areas.
*   _Next_:
    *   Enhance/add tests for hierarchical transitions to explicitly assert the precise order of all entry/exit actions (Task 1 from Huddle 2025-05-16).
    *   Implement tests for guard behavior with multiple candidate transitions (Task 2 from Huddle 2025-05-16).

---

## 2025-05-15 · _Session End_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 02-hierarchy-guards (RISC-V QEMU Runtime Fix & Documentation Planning)
*   _Work_:
    *   Successfully resolved the `just run-rv` QEMU execution error by correcting the `-mon chardev` argument in the workspace `/.cargo/config.toml` runner string (changed `chardev:char0` to `chardev=char0`).
    *   The `traffic_light` example now compiles and runs correctly on the `riscv32imac-unknown-none-elf` target via QEMU, showing semihosting and UART output.
    *   Consolidated Cargo configurations by moving target-specific `rustflags` and `