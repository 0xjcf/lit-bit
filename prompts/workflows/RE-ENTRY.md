```
# Re-Entry Prompt (copy-paste into AI chat)
<re-entry_prompt>
I'm resuming work on the Rust-Statechart project (lit-bit repo).

1. Please read the latest entry in prompts/project/PROGRESS.md and summarise:
   • Date, author, phase, work done, and "Next" item.

2. Open the active phase checklist (look at the phase named in the summary, e.g. prompts/phases/02-hierarchy-guards/02_checklist.md) and:
   a. List the boxes that are still unchecked.
   b. Highlight any line annotated with "⚠ blocked".

3. Check for a task decomposition file for the **current phase** (identified in step 1) in the `prompts/decomposition/` directory (e.g., `NN_<phase-name-slug>_tasks.md`).
   a. If it exists, briefly mention its presence.
   b. If it **does not exist**, propose creating it by breaking down the items from the current phase's `<prefix>_checklist.md` (from step 2) into actionable sub-tasks. List these proposed sub-tasks. *(This new decomposition file should be the first task to complete if missing).* 

4. Based on the checklist (step 2) and the decomposition (step 3, if it existed or was just proposed), propose the top two concrete coding/implementation tasks I should tackle in this session (include file paths and suggested commits). If creating the decomposition file was proposed in step 3b, that should be the first priority.

<end>
```