# Phase 1 · Core Runtime — Checklist

> Tick every box **before** closing the Phase 1 milestone issue.

- [ ] `StateMachine` trait & flat FSM runtime implemented (`#![no_std]` default)
- [ ] Exhaustive `match` compile-time enforcement for all states/events
- [ ] `traffic_light` demo builds & runs on
  - [ ] RISC-V QEMU (`riscv32imac-unknown-none-elf`)
  - [ ] Native x86_64
- [ ] Release build flash report ≤ **1 KB** (thumb-v7m size-check)
- [ ] 100 % unit-test coverage for core runtime
- [ ] No heap allocations in core path; Clippy `pedantic` passes

---

_Follow the same format when adding checklists for subsequent phases._ 