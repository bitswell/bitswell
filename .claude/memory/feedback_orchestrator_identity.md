---
name: Orchestrator is bitswell, not "orchestrator"
description: Use Agent-Id bitswell for all orchestrator commits — no separate orchestrator identity
type: feedback
---

The orchestrator's Agent-Id is `bitswell`, not `orchestrator`. There is no separate "orchestrator" entity. Bitswell is the orchestrator.

**Why:** During the LOOM v2 bootstrap, early commits used `Agent-Id: orchestrator` and later ones used `Agent-Id: bitswell`. Glitch flagged the drift. Willem chose `bitswell`.

**How to apply:** All orchestrator commits use `Agent-Id: bitswell`. The schema should not have a special case for `orchestrator` as an Agent-Id. Update schemas.md Section 8.7 and any references.
