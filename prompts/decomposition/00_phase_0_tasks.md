# Phase 0 · Spec & Foundations — Task Decomposition

> Derived from the [Phase 0 Checklist](../phases/01-planning/00_checklist.md) and [Roadmap Overview](../analysis/ROADMAP_OVERVIEW.md). Break these down further into actionable steps or PRs as needed.

## Key Deliverables
1. **`Spec.md` committed and reviewed.**
   - [ ] Task: Draft initial `Spec.md` covering core concepts, goals, non-goals.
   - [ ] Task: Define and document the `statechart!` macro grammar (EBNF or similar).
   - [ ] Task: Specify public API surface (`StateMachine` trait, event/state enums).
   - [ ] Task: Define error handling strategy (compile-time vs. runtime).
   - [ ] Task: Solicit review of `Spec.md` from core maintainers (@0xjcf).
   - [ ] Task: Address review feedback and finalize `Spec.md`.

2. **Macro grammar frozen.**
   - [ ] Task: Confirm grammar definition in `Spec.md` is final for v0.1.
   - [ ] Task: Add note to `.cursor/rules/statechart.mdc` §3 confirming freeze.

3. **Crate scaffold (`cargo new`) merged to `main`.**
   - [ ] Task: Run `cargo new rust-statechart --lib` (or similar) at repo root.
   - [ ] Task: Configure initial `Cargo.toml` (name, version="0.0.0", edition, authors, license).
   - [ ] Task: Set up basic `src/lib.rs` with crate-level docs placeholder.
   - [ ] Task: Add `.gitignore` for Rust projects.
   - [ ] Task: Commit initial scaffold.

4. **CI skeleton passes (`lint`, `fmt`, `clippy`).**
   - [ ] Task: Create `.github/workflows/ci.yml`.
   - [ ] Task: Add jobs for `rustfmt --check`, `cargo clippy -- -D warnings`, `cargo check` on stable/beta/nightly.
   - [ ] Task: Ensure CI workflow passes on the initial scaffold.

5. **Licensing headers (MIT / Apache 2.0) applied.**
   - [ ] Task: Add `LICENSE-MIT` and `LICENSE-APACHE` files.
   - [ ] Task: Add license boilerplate comment header to `src/lib.rs`.
   - [ ] Task: Add tool/script suggestion for automating header checks (e.g., `reuse`, `licensure`).

6. **All items compile & run under `cargo check` on stable, beta, nightly.**
   - [ ] Task: Verify the CI matrix covers these checks (part of CI skeleton task).

---

*Use this list to guide implementation during Phase 0. Create specific GitHub issues or PRs for larger tasks.* 