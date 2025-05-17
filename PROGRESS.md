# Project Progress Journal

Reverse-chronological log of daily coding sessions.  Keep entries **concise** and link to PRs / issues for full detail.

---

## 2025-05-17 · _Session End (Planned Next: Parallel Runtime Logic)_
*   _Author_: @Gemini (via @user)
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
*   _Author_: @Gemini (via @user)
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
    *   Consolidated Cargo configurations by moving target-specific `rustflags` and `runner` settings from `lit-bit-core/.cargo/config.toml` to the workspace root `/.cargo/config.toml`, ensuring correct linker script processing.
*   _Next_:
    *   Create an initial `README.md` for the `lit-bit` project.
    *   Continue with research prompt for other state machine libraries and address Phase 02 checklist items.

---

## 2025-05-15 · _Session Start_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 02-hierarchy-guards (Core Implementation)
*   _Work_:
    *   Reviewed re-entry prompt and identified next steps for Phase 02.
    *   Implemented initial hierarchical state handling in `lit-bit-core/src/core/mod.rs` (helpers, basic entry/dispatch).
    *   Added `heapless` dependency and resolved clippy lints in `lit-bit-core`.
    *   Designed and agreed on macro syntax for nested states: single `state` keyword, nested freely, `initial:` attribute for composite states.
    *   Updated `prompts/examples/api_usage_showcase.md` to reflect the new macro syntax for hierarchical states.
    *   Added a comprehensive test case (`parse_and_build_hierarchical_showcase_example`) to `lit-bit-macro`.
    *   Confirmed via testing (`cargo test -p lit-bit-macro`) that the existing macro parser and intermediate tree builder in `lit-bit-macro` correctly handle the new hierarchical syntax.
    *   Refactored `Runtime::send` in `lit-bit-core` with LCA-based entry/exit logic: 
        *   Added helpers: `get_path_to_root`, `find_lca` (corrected ancestor logic), `execute_exit_actions_up_to_lca`, `execute_entry_actions_from_lca`, `enter_submachine_to_initial_leaf`.
        *   Updated `Runtime::new` for correct deep initial state entry and `current_state_id` setting.
    *   Added initial tests for hierarchical transitions (`hierarchical_machine_starts_in_correct_initial_leaf_with_entry_actions`, `test_sibling_transition_with_lca`, `test_child_to_parent_transition`, `test_parent_to_child_transition`) in `lit-bit-core`, all passing.
    *   Set up `scripts/lint_app.sh` and updated `Justfile` for improved linting workflow.
*   _Next_: Continue adding targeted tests for more complex hierarchical transition scenarios in `lit-bit-core` (e.g., grandparent transitions, cousin transitions, transitions to other top-level parents). Continue RISC-V linker research in parallel.

---

## 2025-05-15 · _Session End_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 02-hierarchy-guards (Initial Runtime & Macro Refinement)
*   _Work_:
    *   Resolved all linting issues in `lit-bit-macro` and `lit-bit-core`.
    *   Fixed all failing unit tests in `lit-bit-macro` by correcting parsing logic for handler expressions (`Path` to `Expr`) and refining test assertions.
    *   Fixed all failing integration tests in `lit-bit-core` by:
        *   Implementing initial state entry action execution in `Runtime::new`.
        *   Implementing correct entry/exit action logic for state transitions in `Runtime::send`, including conditional execution for self-transitions.
        *   Ensuring `MachineDefinition` is cloned when initializing `Runtime`.
        *   Correcting type visibility for `TestContext` and `TestEvent` in integration tests.
    *   Corrected example (`traffic_light.rs`, `traffic_light_cortex_m.rs`) build issues related to `MachineDefinition::new` signature changes and import paths. `size-check-cortex-m` example now builds.
    *   Identified persistent linker errors for `riscv32imac-unknown-none-elf` target (`run-rv` task) and created a detailed research prompt.
*   _Next_: Conduct research based on the "RISC-V Linker Errors" prompt to resolve undefined symbol issues for the `run-rv` task. Subsequently, proceed with implementing hierarchy and guard features as per Phase 02 checklist.

---

## 2025-05-14 · _Session Start_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 02-hierarchy-guards (Semantic Analysis Stage)
*   _Work_: Discussed and agreed on design for `TmpState` struct and semantic analysis phase. Confirmed error strategy (syn::Error for parsing, compile_error! for semantics), deferred full event enum validation, and backlogged `.foo` shorthand.
*   _Next_: Define `TmpState` struct and skeleton for the recursive builder function that traverses `StateChartInputAst` to populate the `TmpState` tree (Task 3.1 & 3.2 from Huddle).

---

## 2025-05-14 · _Session End (Macro Codegen Complete, Integration Test Started)_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 02-hierarchy-guards (Macro Codegen & Integration Test Setup)
*   _Work_: Completed all planned code generation for `statechart!` macro (StateId, STATES, TRANSITIONS, MachineDefinition const, user-facing struct, StateMachine impl). All macro unit tests (parser, semantic, codegen helpers) passing. Clippy clean. Began integration test in `lit-bit-core/tests/`, defined test context/events/handlers. Encountered persistent `syn` parsing error ("expected `initial`") when macro parses types/paths from integration test scope.
*   _Next_: Research and resolve `syn::parse` issue for types/paths in integration tests. Once resolved, complete and verify the integration test for `statechart!`.

---

## 2025-05-14 · _Session Start_ 
*   _Author_: @JOSΞ (Lead-up to commit 7ca8793d)
*   _Phase_: 02-hierarchy-guards
*   _Work_: Began Phase 2: Established workspace, scaffolded proc-macro crate (`lit-bit-macro`), created Phase 2 planning documents (`prompts/decomposition/02...`, `prompts/phases/02...`).
*   _Next_: Implement procedural macro parser within `lit-bit-macro`.

---

## 2025-05-14 · _Session End (Parser Complete)_
*   _Author_: @JOSΞ (Commit 7ca8793d)
*   _Phase_: 02-hierarchy-guards (Parser Stage)
*   _Work_: Completed and stabilized the procedural macro parser in `lit-bit-macro`. Established workspace structure. Updated `justfile` and relevant prompt files.
*   _Next_: Design `TmpState` structure for semantic analysis.

---

## 2025-05-14 · _Session End (Phase 1 Complete)_ 
*   _Author_: @JOSΞ (Commit 6808002e)
*   _Phase_: 01-core-runtime
*   _Work_: Completed Phase 1: Implemented dual-target runtime (`lit-bit-core`), embedded examples (`traffic_light`, `traffic_light_cortex_m`), and `#![no_std]` build system. Log update itself was commit `f643373e`.
*   _Next_: Begin Phase 2 (Hierarchy & Guards). 

---

## 2025-05-12 · _Session Start_
*   _Author_: @JOSΞ (Commits 96ab4df, 3749ea3c)
*   _Phase_: 00-planning (Spec & Foundations)
*   _Work_: Initial project setup (commit `96ab4df` - scaffold, dev rules, prompts). Then, restructured project to library layout, configured `Cargo.toml`, completed v0.1 `Spec.md` (commit `3749ea3c`).
*   _Next_: Implement CI skeleton.

---

## 2025-05-12 · _Session End (Phase 0 Complete)_
*   _Author_: @JOSΞ (Commit 19333811)
*   _Phase_: 00-planning (Spec & Foundations)
*   _Work_: Completed Phase 0: Added CI skeleton, licenses, finalized project tasks.
*   _Next_: Begin Phase 1 (Core Runtime).

---

_Add new sessions above this line._ 