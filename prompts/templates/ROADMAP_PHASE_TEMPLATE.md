# {{phase_number}} · {{phase_name}} — Prompt Template

> **Purpose**: provide a ready-to-fill prompt for implementing Phase {{phase_number}} as defined in `ROADMAP.md`.

---

## 1 · Phase Goals

_Summarise the high-level focus and new capabilities introduced in this phase._

```
{{phase_focus}}
```

## 2 · Key Deliverables

| # | Deliverable | Success Criteria | Owner |
|---|-------------|------------------|-------|
| 1 |             |                  |       |
| 2 |             |                  |       |

> Use the deliverables column in the roadmap as a starting point.

## 3 · Checklist ▢/▢

- [ ] Item 1
- [ ] Item 2

_Copy checklist items directly from the Phase Completion Checklist in `.cursor/rules/statechart.mdc`._

## 4 · Task Breakdown

1. **Analyse** — identify required code modules, data structures, tests.
2. **Design** — draft public API, diagrams, and approval.
3. **Implement** — code, docs, unit tests.
4. **Review & Merge** — PR reviews and CI passes.

## 5 · Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
|      |        |            |            |

## 6 · Dependencies

* e.g., completion of Phase {{phase_number_minus_one}}

---

> **Template usage**: copy this file into `prompts/phases/{{NN_dir}}/00_phase_brief.md` and replace the `{{…}}` placeholders. 