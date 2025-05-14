# Phase 2 · Hierarchy & Guards — Checklist

> Tick every box **before** closing the Phase 2 milestone issue.

- [ ] Implement parent state and child state concepts.
- [ ] Ensure correct entry/exit action order for nested states during transitions:
    - [ ] Exit actions fire from most specific child to least specific parent.
    - [ ] Entry actions fire from least specific parent to most specific child.
- [ ] Add comprehensive unit tests for parent/child state transition scenarios, covering various nesting levels.
- [ ] Implement guard conditions on transitions.
    - [ ] Guard function takes `(&Context, &Event)` and returns `bool`.
    - [ ] If guard returns `false`, transition does not occur.
    - [ ] If multiple transitions match an event, the first one with a passing guard is taken.
- [ ] Add unit tests for guard conditions:
    - [ ] Test transitions occurring when guard passes.
    - [ ] Test transitions being prevented when guard fails.
    - [ ] Test guard interaction with event matching.
- [ ] Design and implement compile-time error for referencing an unknown state in a transition definition.
    - [ ] This should ideally be a clear error message pointing to the invalid state name.
- [ ] Add a unit test that specifically tries to define a transition to an unknown state and confirms a compile-time failure (or a specific, clear runtime error if compile-time is too complex initially for this specific check).
- [ ] Review and update core runtime for any changes necessitated by hierarchy or guards.
- [ ] Ensure no new heap allocations are introduced in hot paths related to hierarchy/guards.
- [ ] Clippy `pedantic` passes for all new code.

---

_Follow the same format when adding checklists for subsequent phases._ 