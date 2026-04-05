# LOOM Protocol Examples

**Version**: 2.0.0-draft | **Protocol**: `loom/2` | **Status**: Draft

Five complete examples of the LOOM v2 commit-based protocol in action.
All protocol state lives in commit messages and trailers.
No TASK.md, PLAN.md, STATUS.md, or MEMORY.md in any worktree.

---

## Example 1: Simple Task — Single Agent, Completes Successfully

**Scenario**: Orchestrator assigns ratchet to fix a bug. Ratchet implements and completes.

### Step 1: Orchestrator creates assignment

Branch: `loom/ratchet-fix-auth-bug`

```
task(ratchet): fix null pointer in auth middleware

The auth middleware crashes when user.session is nil. This happens
when a logged-out user hits a protected route. Fix the nil check.

File: src/middleware/auth.rs, around line 47.

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: 3-fix-auth-bug
Scope: src/middleware/auth.rs
Dependencies: none
Budget: 50000
```

### Step 2: Ratchet starts work

```
chore(auth): begin fix-auth-bug

Agent-Id: ratchet
Session-Id: f6b392f8-cd1c-47a3-a9fc-c43a4944dfe9
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-03T10:00:00Z
```

### Step 3: Ratchet commits the fix

```
fix(auth): add nil guard before session access

Added early return when user.session is nil. The crash occurred because
the downstream handler assumed session was always present after auth.

Agent-Id: ratchet
Session-Id: f6b392f8-cd1c-47a3-a9fc-c43a4944dfe9
Task-Status: COMPLETED
Files-Changed: 1
Key-Finding: session nil on logged-out users — expected behavior, needs nil guard not restructure
Heartbeat: 2026-04-03T10:04:00Z
```

**Query: check completion**
```bash
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/ratchet-fix-auth-bug
# Output: COMPLETED
```

---

## Example 2: Blocked Task — Agent Waits on Dependency

**Scenario**: Moss is assigned a task that depends on ratchet's schema work. Ratchet is still
running. Moss detects the unmet dependency and blocks.

Branch: `loom/moss-add-user-endpoint`

### Step 1: Orchestrator assignment

```
task(moss): add POST /users endpoint

Add the user creation endpoint. Schema is being migrated by ratchet
(dependency: ratchet/migrate-schema). Wait for that to complete.

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
Task-Status: ASSIGNED
Assigned-To: moss
Assignment: 5-add-user-endpoint
Scope: src/routes/users.rs
Dependencies: ratchet/migrate-schema
Budget: 80000
```

### Step 2: Moss checks dependency, finds it incomplete, blocks

```
chore(users): blocked -- dependency ratchet/migrate-schema not completed

Checked loom/ratchet-migrate-schema. Latest Task-Status is IMPLEMENTING,
not COMPLETED. Cannot proceed without the new schema.

Agent-Id: moss
Session-Id: a2b3c4d5-e6f7-8901-bcde-f01234567890
Task-Status: BLOCKED
Blocked-Reason: dependency loom/ratchet-migrate-schema not yet COMPLETED
Heartbeat: 2026-04-03T11:00:00Z
```

### Step 3: Ratchet completes. Orchestrator unblocks moss.

```
chore(loom): resume moss/add-user-endpoint -- ratchet/migrate-schema now COMPLETED

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
```

### Step 4: Moss resumes

```
chore(users): resume add-user-endpoint

Agent-Id: moss
Session-Id: a2b3c4d5-e6f7-8901-bcde-f01234567890
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-03T11:30:00Z
```

---

## Example 3: Failed Task — Unrecoverable Error

**Scenario**: Drift is assigned to migrate a deprecated API. Discovers the API no longer exists
in any form — task is impossible as specified.

Branch: `loom/drift-migrate-legacy-api`

### Orchestrator assignment

```
task(drift): migrate deprecated /v1/users API to /v2/users

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
Task-Status: ASSIGNED
Assigned-To: drift
Assignment: 7-migrate-legacy-api
Scope: src/routes/
Dependencies: none
Budget: 60000
```

### Drift investigates, cannot proceed

```
chore(routes): failed -- legacy /v1/users API does not exist

Searched the entire codebase. There is no /v1/users route, no legacy
handler, no migration path. The task description references code that
was deleted in commit 3a4b5c6 three months ago. Cannot migrate
something that does not exist.

Agent-Id: drift
Session-Id: b3c4d5e6-f7a8-9012-cdef-012345678901
Task-Status: FAILED
Error-Category: task_unclear
Error-Retryable: false
```

---

## Example 4: Multi-Agent Pipeline — Sequential Dependencies

**Scenario**: Three agents in sequence. Vesper plans, ratchet implements, sable reviews.

### vesper/design-auth-v2 (ASSIGNED -> COMPLETED)

```
task(vesper): design auth v2 architecture

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
Task-Status: ASSIGNED
Assigned-To: vesper
Assignment: 10-design-auth-v2
Scope: docs/architecture/
Dependencies: none
Budget: 100000
```

Vesper commits design doc, completes with:

```
docs(arch): auth v2 design -- JWT with refresh tokens

Agent-Id: vesper
Session-Id: c4d5e6f7-a8b9-0123-def0-123456789012
Task-Status: COMPLETED
Files-Changed: 1
Key-Finding: stateless JWT is the right call -- session storage was the scalability bottleneck
Decision: refresh tokens in httpOnly cookie -- prevents XSS token theft
Heartbeat: 2026-04-03T14:00:00Z
```

### ratchet/implement-auth-v2 (depends on vesper/design-auth-v2)

```
task(ratchet): implement auth v2

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: 11-implement-auth-v2
Scope: src/auth/
Dependencies: vesper/design-auth-v2
Budget: 150000
```

### sable/review-auth-v2 (depends on ratchet/implement-auth-v2)

```
task(sable): review auth v2 implementation

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
Task-Status: ASSIGNED
Assigned-To: sable
Assignment: 12-review-auth-v2
Scope: src/auth/
Dependencies: ratchet/implement-auth-v2
Budget: 80000
```

**Query: check full pipeline status**
```bash
for branch in loom/vesper-design-auth-v2 loom/ratchet-implement-auth-v2 loom/sable-review-auth-v2; do
  status=$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$branch" | head -1 | xargs)
  echo "$branch: $status"
done
```

---

## Example 5: Resource Limit — Agent Exits Gracefully

**Scenario**: Glitch is stress-testing a large codebase. Approaches token budget limit,
commits partial findings, exits cleanly. Orchestrator can spawn a continuation.

Branch: `loom/glitch-stress-test-parser`

### Assignment

```
task(glitch): stress-test the query parser

Find edge cases, invalid inputs, and load-bearing assumptions in
src/parser/. Document all findings in commit trailers.

Agent-Id: bitswell
Session-Id: 9a1b2c3d-e4f5-6789-abcd-ef0123456789
Task-Status: ASSIGNED
Assigned-To: glitch
Assignment: 15-stress-test-parser
Scope: tests/parser/
Dependencies: none
Budget: 60000
```

### Glitch accumulates findings, then hits 90% budget

```
chore(parser): blocked -- resource_limit at 90% token budget

Found 4 edge cases so far. Context approaching limit. Committing
all findings before exit.

Agent-Id: glitch
Session-Id: d5e6f7a8-b9c0-1234-ef01-234567890123
Task-Status: BLOCKED
Blocked-Reason: resource_limit
Key-Finding: empty string input causes infinite loop in tokenizer
Key-Finding: unicode normalization skipped -- NFC vs NFD inputs treated differently
Key-Finding: nested parens beyond depth 32 silently truncated
Key-Finding: NULL bytes in string literals crash the lexer
Heartbeat: 2026-04-03T16:48:00Z
```

The orchestrator spawns a new glitch instance seeded with these findings to continue coverage.

---

*End of LOOM Protocol Examples v2.0.0-draft.*
