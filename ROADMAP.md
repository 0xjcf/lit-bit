Thought for 9 seconds


## Roadmap · Building "Rust-Statechart (XState-grade)" Library

> **Goal:** deliver an ergonomic, type-safe, `no_std`-ready state-chart + minimal actor layer that scales from RISC-V firmware to async web back-ends, matching (or exceeding) XState's feature set.

| Phase                               | Focus                                                                                                     | New Capability                    | Key Requirements                                                                                                                  | Success Criteria                                                                                        |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **0 · Spec & Foundations**          | ● Finalize macro grammar, naming, crate layout. <br>● Decide feature flags (`std`, `async`, `diagram`).   | —                                 | • Spec document covering syntax, public API, error style. <br>• Rust edition & MSRV decided. <br>• MIT / Apache 2.0 dual license. | ✔ Stakeholders sign-off on spec. <br>✔ `cargo check` for empty crate + CI pipeline (lint, fmt, clippy). |
| **1 · Core Runtime (no hierarchy)** | ● Flat FSM engine (single active state). <br>● `StateMachine` trait + compile-time enum states/events.    | Minimal usable FSM.               | • Zero heap (`#![no_std]`). <br>• Exhaustive `match` handling, unit tests.                                                        | ✔ "traffic-light" demo runs on RISC-V QEMU & native. <br>✔ <1 KB code size added (thumb-v7m build).     |
| **2 · Hierarchy & Guards**          | ● Nested states + `Super` bubbling. <br>● Guard + action callbacks.                                       | Full Harel statechart hierarchy.  | • Macro can express `state Parent { state Child { … } }`. <br>• Runtime exits/enters correct order.                               | ✔ 100 % branch-coverage tests for entry/exit order. <br>✔ Compile-time error on unknown state/event.    |
| **3 · Parallel States**             | ● `[parallel]` composite with N independent regions.                                                      | Orthogonal regions.               | • All regions advance independently on an event. <br>• Deterministic exit semantics.                                              | ✔ "Agent" example (Engagement + Knowledge) passes golden trace. <br>✔ No extra heap for unused regions. |
| **4 · Minimal Actor Layer**         | ● Tier-B mailbox (`heapless` / `tokio::mpsc`). <br>● `Addr::try_send` + back-pressure.                    | Safe, single-threaded event loop. | • Generic `Actor<M, N>` in 500 LoC. <br>• `spawn_embassy!` + `spawn_tokio!` helpers.                                              | ✔ Concurrency stress-test shows no re-entrancy panics. <br>✔ Builds & runs without `std`.               |
| **5 · Async & Side-Effects**        | ● `#[cfg(feature="async")]` – async handlers & actions. <br>● Await inside state functions.               | Integrate I/O futures.            | • Uses `async-trait` only when `std` OR `alloc` present. <br>• Actor blocks queue until future resolves.                          | ✔ HTTP-polling demo (Tokio) passes load test. <br>✔ Firmware build still zero-alloc when `async` off.   |
| **6 · Timers & "after" / Delays**   | ● Internal scheduler for delayed events.                                                                  | XState `after` syntax.            | • Uses `embedded-time` on no\_std; `tokio::time` on std.                                                                          | ✔ Blinky demo toggles LED every 500 ms with no drift >1 %.                                              |
| **7 · Invoke / Child Actors**       | ● `invoke child X -> statechart!( … )`. <br>● Automatic forwarding of `done` / `error` events.            | Actor-within-actor hierarchy.     | • Parent restarts or stops children on exit.                                                                                      | ✔ Parent/child fault-injection test recovers correctly.                                                 |
| **8 · Diagram Generation**          | ● `to_dot()` & `to_mermaid()` exposed. <br>● Optional build-time file emit (`diagram = "out/graph.dot"`). | Visual docs.                      | • `TRANSITIONS` const table behind `diagram` feature.                                                                             | ✔ `cargo test --features diagram` emits valid DOT → Graphviz renders without warnings.                  |
| **9 · Tooling & Docs**              | ● CLI: `statechart-gen --diagram target/agent.dot`. <br>● Book-style docs with runnable examples.         | Developer UX.                     | • mdBook site auto-built in CI. <br>• Examples: RISC-V blinky, CMS agent, test-harness tuto.                                      | ✔ `cargo doc --open` shows 95 % public-item docs-coverage.                                              |
| **10 · Public Release**             | ● Crates.io 0.1.0. <br>● Blog-post & diagrams.                                                            | Community adoption.               | • SemVer, changelog, contribution guide. <br>• CI matrix: stable, beta, nightly; cortex-m, riscv32, x86-64.                       | ✔ Passing CI badge. <br>✔ ≥100 GitHub ⭐ within 60 days.                                                 |

### Milestone rhythm

* **2–3 weeks per phase** for core engineering phases (1-7).
* **Continuous examples & tests** after each phase land.
* **Community beta** after Phase 8 (diagram tooling).

### Success KPIs

| KPI                                   | Target by v0.1                                      |
| ------------------------------------- | --------------------------------------------------- |
| Code size (no\_std, Cortex-M0 blinky) | **≤ 4 KB flash**                                    |
| Max RAM overhead                      | **≤ 512 B** for single actor, N=8 queue             |
| Throughput (Tokio, release)           | **≥ 1 M events/s** single-thread                    |
| Compile-time error clarity            | 90 % "unknown-state" mistakes explained in ≤3 lines |
| Docs coverage                         | **≥ 95 %** rustdoc                                  |
| CI matrix passes                      | Stable, beta, nightly; Linux, macOS, Windows        |
| Compile-time performance (large SM)   | **≤ 30 s** to compile a 1 000-state benchmark on `x86_64-unknown-linux-gnu` (release) |

---

**Execution tips**

1. **Lock grammar first** (Phase 0): changes later are breaking.
2. **Eat your own dog food:** convert a CMS agent to the macro after Phase 2; switch it to actor after Phase 4.
3. **Automate benchmarks & size checks** in CI so regressions surface early.
4. **Track compile-time & binary-size budgets** with a dedicated `bench_1000_states` crate; fail CI if compile time >30 s or size increases >10 %.
5. **Deny `unsafe` code by default** and run `clippy --all-targets --all-features -D warnings -D clippy::pedantic`.
6. **Community outreach:** open discussions after Phase 5 to gather feedback before 0.1.
7. **Keep embedded first-class**: every feature must compile under `#![no_std]` (CI job).

Follow this phased roadmap and you'll land a **production-grade, XState-class Rust statechart library** that delights both embedded and backend developers—and positions you for a polished public launch.

<!-- Test comment to trigger pre-commit hook -->
