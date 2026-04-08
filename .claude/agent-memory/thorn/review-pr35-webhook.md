---
name: PR #35 Review — Rust webhook server
description: Stress-test of webhook server for LOOM push-event dispatch. 11 findings, 2 blockers (UTF-8 panic, no timeout). Verdict: request-changes.
type: project
---

PR #35: feat(webhook): Rust webhook server for LOOM push-event dispatch
Branch: claude/refine-local-plan-EDD4o
Verdict: request-changes

**Blockers:**
- P1: UTF-8 truncation panic in trigger.rs — `body[..2000]` panics on multi-byte char boundaries
- P3: No request timeout on TriggerClient — hung connections live forever, compounds with unbounded spawning

**High:**
- P2: No explicit body size limit (relies on axum default)
- P4: Unbounded tokio::spawn with no concurrency control

**Medium:**
- P5: Trailer parser false positives — doesn't validate trailer paragraph structure
- P6: No graceful shutdown — in-flight dispatches lost on deploy

**Why:** This is the reliability bridge for LOOM dispatch. Silent dispatch loss undermines the protocol's core promise.

**How to apply:** P1 and P3 are non-negotiable before merge. Track whether fixes are applied in subsequent commits on this branch.
