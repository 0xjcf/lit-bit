# Project Progress Journal

Reverse-chronological log of daily coding sessions.  Keep entries **concise** and link to PRs / issues for full detail.

---

## 2025-05-15 · _Session End_
*   _Author_: @Gemini (via @0xjcf)
*   _Phase_: 02-hierarchy-guards (Core Implementation & Testing)
*   _Work_:
    *   Added three new complex hierarchical transition tests to `lit-bit-core`:
        *   `test_grandchild_to_grandparent_reentry`
        *   `test_cousin_child_transition`
        *   `test_cross_top_level_parent_transition`
    *   All core tests (14) are passing, validating LCA and entry/exit logic for these new scenarios.
    *   Reviewed open issues from `statig` project for proactive pitfall avoidance.
    *   Generated a research prompt for deeper investigation into other state machine libraries.
    *   Test coverage report generated: `lit-bit-core` at 92.64% lines, `lit-bit-macro` at 91.70% lines. Overall: 91.91% lines.
*   _Next_: Continue with research prompt for other state machine libraries. Address any findings or proceed with Phase 02 checklist items.

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