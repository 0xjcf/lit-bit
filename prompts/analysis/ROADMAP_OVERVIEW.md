# ROADMAP Overview — Analysis Prompt

The purpose of this document is to give a concise, AI-consumable overview of the entire development roadmap defined in `ROADMAP.md`.  Use it to generate task decomposition, estimations, and progress reports.

---

## Phases & Key Deliverables

| # | Phase                               | Focus / New Capability                 | Key Deliverables (extract) |
|---|-------------------------------------|----------------------------------------|----------------------------|
| 0 | Spec & Foundations                  | Grammar, crate scaffold, CI skeleton   | Spec.md, CI pipeline       |
| 1 | Core Runtime (no hierarchy)         | Flat FSM engine                        | `traffic_light` demo, ≤1 KB binary, 100 % unit tests |
| 2 | Hierarchy & Guards                  | Nested states, guards                  | Entry/exit order tests, compile-time unknown-state error |
| 3 | Parallel States                     | Orthogonal regions                     | `agent_parallel` golden trace, memory diff ≤ +200 B |
| 4 | Minimal Actor Layer                 | Single-threaded mailbox actor          | Generic `Actor`, Embassy/Tokio spawns, stress test 100 k events |
| 5 | Async & Side-Effects                | Async handlers                         | HTTP polling demo, zero-alloc when async off |
| 6 | Timers & "after" / Delays           | Delayed events                         | Blinky drift < 1 %         |
| 7 | Invoke / Child Actors               | Child actors lifecycle                 | Restart policy test, parent-child demo |
| 8 | Diagram Generation                  | DOT & Mermaid export                   | `to_dot()` round-trip, docs include Mermaid |
| 9 | Tooling & Docs                      | CLI generator, mdBook site             | 95 % rustdoc, docs published |
|10 | Public Release                      | Community adoption                     | CHANGELOG, SemVer tag, blog post |

---

## Milestone Rhythm

* **2–3 weeks per core phase** (1-7).
* Continuous examples and tests after each phase.
* Community beta after Phase 8.

## Success KPIs (target by v0.1)

* Code size (no_std, Cortex-M0 blinky) ≤ **4 KB** flash
* Max RAM overhead ≤ **512 B** (single actor, N=8 queue)
* Throughput (Tokio, release) ≥ **1 M events/s** single-thread
* Compile-time error clarity: 90 % unknown-state mistakes explained in ≤ 3 lines
* Docs coverage ≥ **95 %** rustdoc
* CI matrix passes: stable, beta, nightly; Linux, macOS, Windows

---

### How to Use This Prompt

1. **Decomposition** – Copy the table above into a decomposition prompt and break each deliverable into bite-sized implementation tasks.
2. **Estimation** – Use the KPIs and milestone rhythm to estimate effort, team allocation, and risk.
3. **Progress Tracking** – Update checklists in `phases/<NN>-*/00_checklist.md` as tasks are completed.

> Keep this file in sync with `ROADMAP.md` whenever the roadmap is updated. 