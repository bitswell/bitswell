---
name: PR #35 webhook server review
description: Review of Rust webhook server for LOOM push-event dispatch — strong receiver, hollow dispatch layer
type: project
---

PR #35 (branch: claude/refine-local-plan-EDD4o) adds a Rust/Axum webhook server for LOOM dispatch.

**Finding**: The server correctly receives and verifies GitHub webhook events but converts structured LOOM trailer data into unstructured natural language prompts before sending to a single Claude RemoteTrigger endpoint. This defeats LOOM's core design of encoding coordination intent in structured commits.

**Why:** The Ultraplan session that created this PR timed out after 30 minutes. It built the receiver layer cleanly but placeholder'd the dispatch layer — referencing a `loom-dispatch.sh` script that doesn't exist, using a single trigger ID for all agents, and extracting only `Task-Status: ASSIGNED` presence without parsing the full trailer vocabulary.

**How to apply:** If this PR is revised, the key architectural question is: is the webhook a relay (forward raw events to the orchestrator) or part of the orchestrator (must understand the full LOOM protocol)? The dispatch layer needs to either extract full trailer data and route to per-agent triggers, or be explicitly scoped as phase-1 ingestion only.
