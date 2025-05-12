# Project Prompts Directory

This directory contains all prompts, templates, and references used in the project. The structure is organized to make it easy to find specific types of documents.

## Directory Structure

```
prompts/
├── analysis/         # Task analysis templates and complexity assessment guides
├── decomposition/    # Task decomposition strategies and templates
├── estimation/       # Effort estimation frameworks and references
├── phases/           # Implementation phase-specific prompts
│   ├── 01-planning/         # Phase implementation & design
│   │   └── diagrams/
│   ├── 02-development/      # Phase implementation
│   ├── 03-testing/          # Phase implementation
│   ├── 04-deployment/       # Phase implementation
├── project/          # Project management and workflow documents
├── templates/        # Input and output templates
└── workflows/        # Process documentation and workflows
```

## Integration with Roadmap

This repository includes a top‐level `ROADMAP.md` that drives planning and execution.  
The prompts in this folder should **mirror the phases and deliverables described in the roadmap** so that contributors (human or AI) can quickly pick up the next actionable item.

*Every time the roadmap changes, update or regenerate the related prompts so that they stay in sync.*

### Recommended Prompt Artefacts

1. `analysis/ROADMAP_OVERVIEW.md` — High-level summary of all phases and key deliverables.
2. `templates/ROADMAP_PHASE_TEMPLATE.md` — A generic "phase prompt" template.  Copy it to a phase folder (e.g. `phases/02-development/01_phase_brief.md`) and fill in the details.
3. `phases/<NN>-*/` — Concrete prompts for the current milestone.  At minimum each phase should contain:
   * `00_checklist.md` — Phase completion checklist (one-to-one with the roadmap table).
   * `01_tasks.md` — Break-down of tasks / user-stories for the phase.
   * `diagrams/` — Any supporting diagrams (architecture, sequence, state-chart).

> **Tip:** Use `just sync-prompts` (to be added) to regenerate boilerplate files from `templates/`.

## Example Phase Directory

```
phases/02-development/
├── 00_checklist.md   # Auto-generated from ROADMAP.md
├── 01_tasks.md       # Decomposed implementation tasks
└── diagrams/
    └── traffic_light_state.svg
```

Keeping these artefacts tidy and up-to-date ensures we always have an accurate source of truth that matches the implementation roadmap.


## Key Files (Original from dev-setup, review if still needed)

### Project Management

- `project/PROJECT_WORKFLOW.md` - Development workflow guidelines and templates
- `project/PROGRESS.md` - Phase implementation progress tracking (We are using this one)
- `project/PHASE_ASSESSMENT.md` - Assessment of phase readiness
- `project/NEXT_PHASE.md` - Guide for initiating subsequent phases/tasks.

### Implementation Phases

Each phase directory contains detailed prompts for implementing that phase. If created using `--create-placeholders`, common starting prompts may exist:
*   **Phase 01 (Planning/Design):** Typically includes design documents (`01_domain_model.md`, `02_architecture_design.md`) and diagrams (`diagrams/`).
*   **Phase 02 (Development):** Focuses on implementation prompts (`01_<feature>_implementation.md`), guided by Phase 01 designs.
*   **Phase 03 (Testing):** Contains testing definition prompts (`01_test_plan.md`, `02_e2e_test_cases.md`, `03_accessibility_audit.md`).

## Using These Prompts (Original from dev-setup, review if still needed)

1. Start with the project management files to understand the overall workflow.
2. Review the phase-specific prompts for planning and implementation details.
3. Use the templates and guides during development for consistent approaches.

For adding new prompts, please follow the established directory structure and naming conventions.
