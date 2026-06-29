---
name: review
description: Self-review a diff before committing — correctness, edge cases, and tests.
---

# Review the diff

Read the whole diff as if someone else wrote it.

- **Correctness**: does it actually do what the plan asked? Trace one real input end to end.
- **Edge cases**: empty input, errors, nulls, concurrency, large data.
- **Blast radius**: did any public API, schema, or shared helper change in a way that breaks callers?
- **Tests**: is the new behavior covered? Run the test suite and report the result honestly.
- **Cleanup**: no stray debug logs, commented-out code, or unrelated churn.

Report findings plainly. If something is wrong or untested, say so before committing.
