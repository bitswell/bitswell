---
name: Tests should be in Rust
description: Default to Rust for all testing unless explicitly told otherwise
type: feedback
---

Tests should be written in Rust, not shell scripts or Python.

**Why:** User preference — Rust is the project's primary language and Willem wants consistency.

**How to apply:** When building test harnesses, probes, or validation logic (e.g., issue #39 test framework), write them as Rust binaries or `#[test]` functions. Only use shell/Python if the user explicitly asks for it.
