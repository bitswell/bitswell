---
name: Agent pipeline — issue to merge workflow
description: The full pipeline from bitsweller issue to merged implementation, and current state of first pipeline run
type: project
---

The agent team operates a pipeline: Issue (bitsweller) -> Plan (vesper) -> Implement (ratchet/moss) -> Review (drift/sable/thorn/glitch) -> Approve (bitswelt).

**Why:** Separation of concerns — each agent has a distinct perspective. Writers don't review their own work. The approver traces the full arc before sign-off.

**How to apply:** When running the pipeline, launch reviewers in parallel (they're independent). The approver runs last. PR #5 is the first real pipeline run — it established that 3 reviewers is the practical default (drift + sable + thorn), with glitch reserved for implementation changes that need chaos testing.

**First pipeline observation (PR #5):** Drift and Sable both flagged that bitswell/bitsweller/bitswelt skipped identity discovery while the other 7 agents went through it. This is a known gap — these three are infrastructure agents whose identities were assigned, not discovered.
