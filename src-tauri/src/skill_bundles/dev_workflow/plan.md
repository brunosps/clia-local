---
name: plan
description: Plan a code change before writing it — clarify scope, find the files, and outline steps.
---

# Plan a change

Before writing any code, produce a short, concrete plan.

1. **Restate the goal** in one sentence so the intent is unambiguous.
2. **Map the code**: list the files and functions the change will touch. Prefer reusing
   existing utilities over adding new ones.
3. **Outline the steps** as a numbered checklist, smallest safe increments first.
4. **Call out risks**: data migrations, public APIs, concurrency, anything hard to reverse.
5. **Define done**: how the change will be verified (tests to run, manual check).

Keep the plan tight. Do not start editing until the plan is clear.
