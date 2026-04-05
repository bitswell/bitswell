# LOOM Examples

**Version**: 2.0.0-draft | **Protocol**: `loom/2` | **Status**: Draft

Five concrete examples of LOOM v2 patterns. Each example shows complete commit
sequences with all required trailers. All protocol state lives in commits.

See `schemas.md` for the full trailer vocabulary. See `worker-template.md` for the
step-by-step agent guide.

---

## Example 1: Simple Task — Assign, Implement, Complete

The simplest case. The orchestrator assigns a task; the agent implements it
without interruption; the branch is ready for integration.

**Branch:** `loom/ratchet-add-commit-schema`

**Actors:** bitswell (orchestrator), ratchet (agent)

---

**Commit 1** — Orchestrator assigns the task

```
task(ratchet): define LOOM commit schema and trailer vocabulary (#12)

Define the canonical commit message format and trailer vocabulary for
LOOM v2. This is the source of truth for all agents and the orchestrator.

Acceptance criteria:
- Conventional Commits format documented
- All trailers defined: type, required/optional, per-state rules
- AGENT.json schema with all fields and constraints
- State extraction queries (git log --format examples)
- Validation rules for CI

Agent-Id: bitswell
Session-Id: b1e2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: 2-commit-schema
Scope: .claude/skills/loom/references/schemas.md
Dependencies: none
Budget: 100000
```

---

**Commit 2** — Agent signals start

```
chore(loom): begin commit-schema

Agent-Id: ratchet
Session-Id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-01T10:00:00Z
```

---

**Commit 3** — Agent makes progress (file change)

```
docs(loom): add AGENT.json schema and branch naming convention

Agent-Id: ratchet
Session-Id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Heartbeat: 2026-04-01T10:04:00Z
Key-Finding: dispatch.trigger_ref is only meaningful when mode is push-event; made it conditional
```

---

**Commit 4** — Agent completes

```
docs(loom): add trailer vocabulary, state requirements, and extraction queries

All sections complete. Validation rules marked as "not yet enforced" to
set expectations — tooling does not exist yet.

Agent-Id: ratchet
Session-Id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: COMPLETED
Files-Changed: 1
Key-Finding: dispatch.trigger_ref is only meaningful when mode is push-event; made it conditional
Key-Finding: validation rules marked not-yet-enforced — tooling does not exist yet
Decision: mark validation as not-yet-enforced rather than omitting -- preserves normative intent without false promises
Heartbeat: 2026-04-01T10:11:00Z
```

**Query: verify completion**

```bash
git log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' \
  loom/ratchet-add-commit-schema
# Output: COMPLETED
```

---

## Example 2: Long Task — Heartbeat Checkpoints

A task that takes more than 5 minutes. The agent commits heartbeat checkpoints
to prove liveness even when it has no file changes ready to commit.

**Branch:** `loom/moss-migrate-identities`

---

**Commit 1** — Assignment (by orchestrator, abbreviated)

```
task(moss): migrate agent identities to .mcagent/agents/ (#18)

...

Agent-Id: bitswell
Session-Id: b1e2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: ASSIGNED
Assigned-To: moss
Assignment: 4-migrate-identities
Scope: .mcagent/agents/**
Dependencies: none
Budget: 80000
```

---

**Commit 2** — Start

```
chore(mcagent): begin migrate-identities

Agent-Id: moss
Session-Id: c1d2e3f4-a5b6-7890-abcd-ef1234567890
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-01T11:00:00Z
```

---

**Commit 3** — Checkpoint (no file changes)

```
chore(loom): checkpoint

Reading existing agent files. Taking longer than expected — eight
identity files with non-trivial content to parse and reorganize.

Agent-Id: moss
Session-Id: c1d2e3f4-a5b6-7890-abcd-ef1234567890
Heartbeat: 2026-04-01T11:05:00Z
```

---

**Commit 4** — Partial work committed

```
feat(mcagent): migrate ratchet, moss, drift identities

Agent-Id: moss
Session-Id: c1d2e3f4-a5b6-7890-abcd-ef1234567890
Heartbeat: 2026-04-01T11:09:00Z
Key-Finding: identity files vary in format; normalized to standard Markdown headers
```

---

**Commit 5** — Another checkpoint

```
chore(loom): checkpoint

Agent-Id: moss
Session-Id: c1d2e3f4-a5b6-7890-abcd-ef1234567890
Heartbeat: 2026-04-01T11:14:00Z
```

---

**Commit 6** — Completion

```
feat(mcagent): migrate remaining five agent identities

All eight agents migrated. Directory structure matches mcagent-spec.md
Section 3 exactly.

Agent-Id: moss
Session-Id: c1d2e3f4-a5b6-7890-abcd-ef1234567890
Task-Status: COMPLETED
Files-Changed: 8
Key-Finding: identity files vary in format; normalized to standard Markdown headers
Key-Finding: two agents (bitsweller, bitswelt) had no prior identity file; stubs created
Decision: created stubs for missing identities rather than skipping -- empty slot is confusing
Heartbeat: 2026-04-01T11:22:00Z
```

**Query: all findings from this agent**

```bash
git log --format='%(trailers:key=Key-Finding,valueonly)' \
  loom/moss-migrate-identities | grep -v '^$'
# Output:
# identity files vary in format; normalized to standard Markdown headers
# identity files vary in format; normalized to standard Markdown headers
# two agents (bitsweller, bitswelt) had no prior identity file; stubs created
```

*(Key-Finding appears twice for the first finding because it appeared in two commits.)*

---

## Example 3: Blocked and Unblocked

An agent hits a blocker mid-task, signals it, and is resumed by the orchestrator.
The branch has two IMPLEMENTING arcs separated by a BLOCKED commit.

**Branch:** `loom/drift-review-schemas`

---

**Commit 1** — Assignment

```
task(drift): review commit schema for correctness and completeness (#15)

Review schemas.md against actual agent usage. Report findings as
Key-Finding trailers. Suggest amendments if needed.

Agent-Id: bitswell
Session-Id: b1e2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: ASSIGNED
Assigned-To: drift
Assignment: 3-review-schemas
Scope: .claude/skills/loom/references/schemas.md
Dependencies: ratchet/commit-schema
Budget: 60000
```

---

**Commit 2** — Start

```
chore(loom): begin review-schemas

Agent-Id: drift
Session-Id: d1e2f3a4-b5c6-7890-abcd-ef1234567890
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-01T12:00:00Z
```

---

**Commit 3** — Blocked (dependency not ready)

```
chore(loom): blocked -- dependency ratchet/commit-schema not completed

Checked loom/ratchet-add-commit-schema: Task-Status is IMPLEMENTING,
not COMPLETED. Cannot review a schema that is still being written.
Waiting for ratchet to complete.

Agent-Id: drift
Session-Id: d1e2f3a4-b5c6-7890-abcd-ef1234567890
Task-Status: BLOCKED
Blocked-Reason: dependency ratchet/commit-schema not yet COMPLETED
Heartbeat: 2026-04-01T12:03:00Z
```

---

**Commit 4** — Orchestrator resolves blocker (post on drift's branch)

```
chore(loom): unblock -- ratchet/commit-schema now COMPLETED

ratchet completed schemas.md at commit a1b2c3d. Dependency is met.
Resume review.

Agent-Id: bitswell
Session-Id: b1e2c3d4-e5f6-7890-abcd-ef1234567890
```

*(No `Task-Status` trailer — this is an orchestrator unblock commit, not a state change.)*

---

**Commit 5** — Agent resumes (new IMPLEMENTING arc)

```
chore(loom): resume review-schemas

Dependency resolved. Resuming review of schemas.md.

Agent-Id: drift
Session-Id: d1e2f3a4-b5c6-7890-abcd-ef1234567890
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-01T13:00:00Z
```

---

**Commit 6** — Completion

```
docs(loom): review schemas.md — findings and suggested amendments

Two amendments suggested inline. Three findings recorded.

Agent-Id: drift
Session-Id: d1e2f3a4-b5c6-7890-abcd-ef1234567890
Task-Status: COMPLETED
Files-Changed: 1
Key-Finding: BLOCKED commit validation rule missing from Section 8 — Blocked-Reason not listed as required
Key-Finding: Heartbeat SHOULD vs MUST inconsistency between Section 4.2 and Section 5.2
Key-Finding: state machine diagram omits BLOCKED->IMPLEMENTING arc
Decision: amended file directly rather than filing new issues -- faster and within scope
Heartbeat: 2026-04-01T13:18:00Z
```

---

## Example 4: Task With Dependencies

An agent waits for two upstream agents before starting work. Shows how
to check dependency status and self-block correctly.

**Branch:** `loom/vesper-plan-integration`

---

**Commit 1** — Assignment

```
task(vesper): plan integration order for PR #20 (#22)

Given the completed work from ratchet (schemas) and moss (identities),
plan the integration sequence. Produce a markdown integration plan at
tests/loom-eval/integration-plan.md.

Agent-Id: bitswell
Session-Id: b1e2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: ASSIGNED
Assigned-To: vesper
Assignment: 5-plan-integration
Scope: tests/loom-eval/integration-plan.md
Dependencies: ratchet/commit-schema, moss/migrate-identities
Budget: 50000
```

---

**Commit 2** — Agent checks deps; both are COMPLETED; starts immediately

```
chore(eval): begin plan-integration

Verified both dependencies:
  loom/ratchet-add-commit-schema -> COMPLETED
  loom/moss-migrate-identities   -> COMPLETED

Agent-Id: vesper
Session-Id: e1f2a3b4-c5d6-7890-abcd-ef1234567890
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-02T09:00:00Z
```

---

**Commit 3** — Completion

```
docs(eval): add integration plan for PR #20

Integration order: ratchet first (no deps), then moss (no deps),
then drift (depends on ratchet), then vesper (depends on both).
Validation gate between each integration.

Agent-Id: vesper
Session-Id: e1f2a3b4-c5d6-7890-abcd-ef1234567890
Task-Status: COMPLETED
Files-Changed: 1
Key-Finding: ratchet and moss are independent -- can be integrated in parallel if workspace allows
Key-Finding: drift depends only on ratchet, not moss -- can start after ratchet integrates
Deviation: suggested parallel integration -- task asked for sequence only -- orchestrator should decide
Heartbeat: 2026-04-02T09:14:00Z
```

**Query: check all dependencies before integrating**

```bash
for dep in ratchet/commit-schema moss/migrate-identities; do
  branch="loom/${dep/\//'-'}"  # ratchet/commit-schema -> loom/ratchet-commit-schema
  status=$(git log -1 \
    --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$branch")
  echo "$dep: $status"
done
# Output:
# ratchet/commit-schema: COMPLETED
# moss/migrate-identities: COMPLETED
```

---

## Example 5: Failed Task

An agent encounters an unrecoverable error and signals FAILED. The
branch is preserved; the orchestrator decides whether to retry.

**Branch:** `loom/glitch-validate-hooks`

---

**Commit 1** — Assignment

```
task(glitch): validate all git hooks fire correctly in CI (#31)

Run the hook validation suite in ci/hooks/. All 12 hooks must fire and
produce expected output. Report any failures.

Agent-Id: bitswell
Session-Id: b1e2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: ASSIGNED
Assigned-To: glitch
Assignment: 7-validate-hooks
Scope: ci/hooks/**
Dependencies: none
Budget: 40000
```

---

**Commit 2** — Start

```
chore(ci): begin validate-hooks

Agent-Id: glitch
Session-Id: f1a2b3c4-d5e6-7890-abcd-ef1234567890
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-03T14:00:00Z
```

---

**Commit 3** — Progress, then discovery of blocker

```
chore(ci): checkpoint -- hook runner requires network access

Ran hooks 1-4 successfully. Hook 5 (remote-ref-check) requires
network access to validate upstream refs. The sandbox blocks outbound
connections. Cannot proceed without orchestrator assist or scope change.

Agent-Id: glitch
Session-Id: f1a2b3c4-d5e6-7890-abcd-ef1234567890
Heartbeat: 2026-04-03T14:07:00Z
Key-Finding: hooks 1-4 pass; hook 5+ require network access blocked by sandbox
```

---

**Commit 4** — Failed (unretryable in current environment)

```
chore(ci): failed -- network access required, blocked by sandbox

Hooks 1-4 validated successfully. Hooks 5-12 all require network
access (remote ref validation, push authentication checks). The sandbox
environment blocks all outbound connections. This is an environment
constraint, not a code bug.

Retry will fail under the same conditions. Orchestrator must either:
  (a) run this task outside the sandbox, or
  (b) split the task to validate only non-network hooks (1-4)

Agent-Id: glitch
Session-Id: f1a2b3c4-d5e6-7890-abcd-ef1234567890
Task-Status: FAILED
Error-Category: blocked
Error-Retryable: false
```

*(No `Heartbeat` trailer — FAILED commits do not require it, per schemas.md Section 5.5.)*

**Query: get the error details**

```bash
git log -1 \
  --format='Task-Status: %(trailers:key=Task-Status,valueonly)%nError-Category: %(trailers:key=Error-Category,valueonly)%nError-Retryable: %(trailers:key=Error-Retryable,valueonly)' \
  --grep='Task-Status:' \
  loom/glitch-validate-hooks
# Output:
# Task-Status: FAILED
# Error-Category: blocked
# Error-Retryable: false
```

---

*End of LOOM Examples v2.0.0-draft.*
