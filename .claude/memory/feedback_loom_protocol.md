---
name: Use LOOM protocol for all implementation work
description: Never edit loom-tools files directly — dispatch agents through LOOM worktrees
type: feedback
---

All implementation work on loom-tools (and likely other repos) must go through the LOOM protocol. Don't edit files directly — spawn agents into worktrees and let them do the work.

**Why:** The whole point of loom-tools is agent coordination via worktrees. Bitswell (the orchestrator) should orchestrate, not implement directly. Direct edits bypass the protocol we're building.

**How to apply:** When continuing work on loom-tools phases, use the `/loom` skill to dispatch writer agents (ratchet, moss) into the appropriate phase worktrees. Bitswell coordinates: assigns, reviews, merges. The worktrees already exist at `repos/loom-tools-phase-N/`.
