---
name: implement
description: Implement an approved plan — make focused edits that match the surrounding code.
---

# Implement the plan

Execute the plan one step at a time.

- **Match the codebase**: follow the existing naming, structure, and idioms of the files you edit.
- **Small commits of change**: keep each edit focused on a single step; don't mix refactors with features.
- **No dead scope creep**: only touch what the plan calls for. Note anything extra you discover for later.
- **Keep it runnable**: after each step the project should still type-check / compile.
- **Update the obvious neighbors**: types, call sites, and tests that the change forces.

When a step is done, move to the next; when all steps are done, hand off to `review`.
