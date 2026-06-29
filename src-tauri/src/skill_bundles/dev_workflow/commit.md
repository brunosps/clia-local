---
name: commit
description: Write a clean conventional commit and a clear PR description.
---

# Commit and open a PR

Only commit once the change is verified.

1. **Stage intentionally**: include only the files that belong to this change.
2. **Conventional message**: `type(scope): summary` — e.g. `feat(queue): add archive action`.
   Types: feat, fix, refactor, docs, test, chore, ci.
3. **Body (why, not what)**: one or two lines explaining the motivation and any trade-offs.
4. **PR description**: what changed, how it was verified, and anything reviewers should focus on.
5. **Never commit** secrets, large binaries, or unrelated files.

Confirm the working tree is clean and the build is green before you push.
