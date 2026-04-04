# LOOM Evaluation Plan v13 -- Observability Audit

## Goal

Evaluate whether LOOM produces sufficient observability artifacts for a human operator to monitor a live run and debug a completed (or failed) run after the fact. This plan treats LOOM as an operations system and asks: can I reconstruct what happened, when, and why -- using only git and the files LOOM commits?

Unlike plan-v0, which audits the spec for internal consistency, this plan audits the *runtime trace* that a LOOM execution leaves behind. Every agent in this evaluation will simulate, inject, or inspect observability signals rather than doing "real" work.

## Threat Model for Observability

A human operator needs to answer these questions after a LOOM run:

1. **Liveness**: Was every agent alive throughout its run, or did one stall silently?
2. **Ordering**: In what order did state transitions actually occur?
3. **Attribution**: Which agent wrote which commit? Can I trust the trailers?
4. **Completeness**: Did every agent reach a terminal state, or were some abandoned?
5. **Forensics**: If something went wrong, can I find the root cause from git alone?
6. **Retention**: Are failed branches preserved so I can investigate later?

The agents below each attack one or more of these questions.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|--------------|
| `heartbeat-enforcer` | Tester | Validate heartbeat timing: parse STATUS.md commits, compute intervals, flag gaps > 5 min. Build a tool that audits any loom branch for heartbeat compliance. | none |
| `status-parser` | Tester | Validate STATUS.md parsing robustness: generate valid, edge-case, and malformed STATUS.md files, parse them, report which survive YAML parsing. Verify every required field is present per state. | none |
| `git-trail-auditor` | Tester | Validate the git log as an audit trail: check commit trailer presence, verify Agent-Id/Session-Id on every commit, reconstruct the full state-machine timeline from git log alone. | none |
| `branch-retention` | Tester | Validate branch retention policy: simulate FAILED agents, verify branches are retained, test that cleanup scripts skip failed branches, and that the 30-day retention rule is enforceable from git metadata. | none |
| `operator-dashboard` | Integrator | Consume output from all 4 audit agents. Build a single-file operator observability report showing: overall health, per-agent timeline, compliance violations, and recommendations. | `heartbeat-enforcer`, `status-parser`, `git-trail-auditor`, `branch-retention` |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `heartbeat-enforcer` | `tests/loom-eval/heartbeat-enforcer/**` | `[]` |
| `status-parser` | `tests/loom-eval/status-parser/**` | `[]` |
| `git-trail-auditor` | `tests/loom-eval/git-trail-auditor/**` | `[]` |
| `branch-retention` | `tests/loom-eval/branch-retention/**` | `[]` |
| `operator-dashboard` | `tests/loom-eval/operator-dashboard/**` | `[]` |

No scope overlap -- all five agents write to isolated directories.

## Detailed Agent Tasks

### Agent: `heartbeat-enforcer`

**Objective**: Determine whether a human operator can detect a stalled agent by inspecting heartbeat commits.

**Audit checks**:

1. **Heartbeat interval measurement**: Write a script that, given a loom branch name, walks `git log` and extracts every `heartbeat_at` value from STATUS.md commits. Compute the delta between consecutive heartbeats. Flag any gap exceeding 5 minutes (the protocol-specified maximum at Section 9.1 / Level 1 rule 5).

2. **Heartbeat-at vs commit timestamp**: Compare the `heartbeat_at` field inside STATUS.md against the git commit's `AuthorDate`. Flag discrepancies greater than 60 seconds -- this detects agents that write a fake future heartbeat without actually committing promptly.

3. **Missing heartbeat in terminal commits**: Verify that the final COMPLETED or FAILED commit still contains a valid `heartbeat_at` that is within 5 minutes of `updated_at`. An agent that completes but forgot to update heartbeat on the last commit would appear stale to a monitoring system.

4. **No-commit stall detection**: Document the blind spot: if an agent stalls without committing, the heartbeat mechanism cannot detect it until `timeout_seconds` elapses. Measure the worst-case detection latency (timeout_seconds + orchestrator polling interval). Recommend a mitigation (e.g., filesystem-level heartbeat file outside git).

5. **Heartbeat during PLANNING vs IMPLEMENTING**: Verify that heartbeat commits occur in both phases, not just during implementation. The protocol says "while running" without phase qualification -- confirm this applies to planning.

**Deliverables**: A shell script `check-heartbeat.sh` that accepts a branch name and outputs a compliance report. A findings document with pass/fail for each check.

### Agent: `status-parser`

**Objective**: Determine whether a human operator can reliably parse STATUS.md to determine agent state, and whether edge cases cause silent misreadings.

**Audit checks**:

1. **YAML front matter extraction**: Generate STATUS.md files with varying whitespace, quoting styles, and comment placement. Verify that a standard YAML parser (PyYAML, yq, or equivalent) extracts all fields correctly. Test at least:
   - Unquoted vs single-quoted vs double-quoted string values
   - Status values in lowercase vs UPPERCASE (protocol says uppercase)
   - ISO-8601 timestamps with and without the trailing `Z`
   - Timestamps with timezone offsets (`+00:00`) instead of `Z`
   - Multi-line `summary` field (should this be rejected?)
   - `error.message` containing YAML-special characters (`:`, `#`, `{`, `}`)

2. **State-conditional field validation**: For each of the 5 states (PLANNING, IMPLEMENTING, COMPLETED, BLOCKED, FAILED), generate a STATUS.md with exactly the required fields and verify it passes. Then remove one required field at a time and verify the parser reports an error. Specifically:
   - FAILED without `error` block
   - FAILED with `error` but missing `error.category`
   - BLOCKED without `blocked_reason`
   - COMPLETED without `files_changed`
   - Any state without `heartbeat_at`

3. **Markdown body after front matter**: Verify that content after the closing `---` does not interfere with YAML parsing. Test STATUS.md files with markdown body containing `---` horizontal rules (potential delimiter confusion).

4. **Round-trip fidelity**: Write a STATUS.md, parse it, re-serialize it, parse again. Confirm all fields survive the round trip. This tests whether an orchestrator that reads and re-writes STATUS.md (e.g., during a status check) would corrupt it.

5. **Empty and missing STATUS.md**: Document what an operator should see when STATUS.md does not exist (agent crashed before first commit) or exists but is empty. Recommend a standard error message.

**Deliverables**: A test suite of STATUS.md fixtures (valid and invalid), a parser validation script, and a findings document.

### Agent: `git-trail-auditor`

**Objective**: Determine whether `git log` alone is a sufficient audit trail to reconstruct a LOOM run.

**Audit checks**:

1. **Trailer presence on every commit**: Walk all commits on a loom branch. For each commit, verify that both `Agent-Id` and `Session-Id` trailers are present in the commit message. Report any commit missing either trailer. Use `git log --format='%(trailers:key=Agent-Id)'` to extract trailers programmatically.

2. **Trailer consistency**: Verify that all commits on a single branch share the same `Agent-Id` and `Session-Id` values (a single agent invocation should not change session mid-run). Flag commits where these values differ, as they could indicate contamination from another agent.

3. **State transition reconstruction**: Using only `git log` and the STATUS.md content at each commit, reconstruct the full state machine path for an agent (e.g., PLANNING -> IMPLEMENTING -> COMPLETED). Verify:
   - The sequence follows a valid path in the transition table (protocol Section 3).
   - No invalid transitions occurred (e.g., PLANNING -> COMPLETED, skipping IMPLEMENTING).
   - Terminal states (COMPLETED, FAILED) are the last status committed.

4. **Orchestrator vs agent commit attribution**: Identify which commits were made by the orchestrator (Agent-Id: orchestrator) vs the worker agent. Verify the orchestrator only committed TASK.md, AGENT.json, and feedback -- never implementation files. Verify the agent never committed outside its scope.

5. **Timeline reconstruction**: Extract commit timestamps and build a chronological timeline across all loom branches in a run. Produce a human-readable timeline showing:
   - Agent spawn times (first commit on each branch)
   - State transitions with timestamps
   - Integration merges (merge commits on the workspace branch)
   - Total wall-clock duration per agent

6. **Session-Id uniqueness across agents**: Verify that no two agents share the same Session-Id. This is critical for disambiguation -- if two agents had the same Session-Id, audit trail queries filtering by Session-Id would return mixed results.

**Deliverables**: A shell script `audit-git-trail.sh` that accepts a workspace branch and outputs a full audit report. A findings document.

### Agent: `branch-retention`

**Objective**: Determine whether failed branches are preserved for post-mortem investigation and whether the 30-day retention policy is enforceable.

**Audit checks**:

1. **Failed branch survival after cleanup**: Simulate a LOOM run where one agent fails and the orchestrator runs `git worktree remove`. Verify that the branch (`loom/<agent-id>`) still exists after worktree removal. Document the distinction between worktree removal (which removes the directory) and branch deletion (which removes the ref).

2. **Retention metadata availability**: After a failure, can an operator determine when the branch was created and when the agent failed? Check whether:
   - The branch creation date is recoverable from `git reflog show loom/<agent-id>`
   - The failure timestamp is in the last STATUS.md commit on the branch
   - The error category and message are recoverable from the branch's STATUS.md

3. **30-day enforcement mechanism**: The protocol specifies 30-day retention but provides no enforcement mechanism. Audit whether:
   - Git itself provides any branch age metadata (reflog expiry defaults to 90 days)
   - A cleanup script could determine branch age from the last commit date
   - There is a risk of accidental deletion via `git branch -D` (no safeguard exists)
   - Recommend a tagging convention (e.g., `refs/loom/failed/<agent-id>/<date>`) to protect retained branches

4. **Branch namespace pollution**: After many LOOM runs, the branch namespace fills with `loom/*` branches. Audit whether:
   - An operator can distinguish active from retained-failed from completed-and-merged branches
   - There is a way to list only failed branches (answer: grep STATUS.md on each branch tip)
   - Recommend a naming convention or metadata tag to distinguish branch states

5. **Worktree orphan detection**: If the orchestrator crashes mid-run, worktrees may be left behind without a controlling process. Verify that `git worktree list` shows orphaned worktrees and that an operator can determine which worktrees belong to which LOOM run (via AGENT.json session_id).

**Deliverables**: A test script that simulates failure scenarios and verifies retention, a findings document with recommendations.

### Agent: `operator-dashboard`

**Objective**: Synthesize findings from all four audit agents into a single observability assessment.

**Depends on**: `heartbeat-enforcer`, `status-parser`, `git-trail-auditor`, `branch-retention`.

**Tasks**:

1. Read MEMORY.md from each of the four audit agents.
2. Compile findings into a structured report with sections:
   - **Observability Scorecard**: Pass/fail summary for each audit check across all agents.
   - **Critical Gaps**: Observability failures that would prevent an operator from debugging a production LOOM run.
   - **Per-Signal Assessment**: For each observability signal (heartbeat, STATUS.md, git trail, branch retention), a rating of Sufficient / Partial / Insufficient with justification.
   - **Operator Runbook Draft**: A short procedure an operator would follow to investigate a stalled or failed LOOM run, based on the signals that actually work.
   - **Recommendations**: Prioritized list of protocol improvements to close observability gaps.
3. Write the report to `tests/loom-eval/operator-dashboard/observability-report.md`.

## Execution Flow

```
Step 1: Create 5 worktrees + branches
        git worktree add .worktrees/heartbeat-enforcer -b loom/heartbeat-enforcer
        git worktree add .worktrees/status-parser      -b loom/status-parser
        git worktree add .worktrees/git-trail-auditor  -b loom/git-trail-auditor
        git worktree add .worktrees/branch-retention   -b loom/branch-retention
        git worktree add .worktrees/operator-dashboard -b loom/operator-dashboard

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.

Step 3: PLANNING PHASE -- spawn 4 audit agents in parallel
        heartbeat-enforcer, status-parser, git-trail-auditor, branch-retention
        all write PLAN.md

Step 4: PLAN GATE -- orchestrator reads all 4 PLAN.md files
        Check for scope overlaps (none expected), unrealistic approaches,
        missing coverage of audit checks listed above. Approve or send feedback.

Step 5: IMPLEMENTATION PHASE -- re-spawn 4 audit agents in parallel
        Each builds its test fixtures, scripts, and findings documents.

Step 6: INTEGRATE -- merge all 4 in any order (no deps between them)
        Validate after each merge.

Step 7: PLAN + IMPLEMENT operator-dashboard
        (depends on all 4 being integrated)
        Reads 4 MEMORY.md files, compiles observability report.

Step 8: INTEGRATE operator-dashboard. Clean up all worktrees.
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 5 agents, 5 worktrees, non-overlapping scopes |
| Parallel planning | 4 agents plan simultaneously |
| Plan gate | Orchestrator reviews 4 plans before any implementation |
| Parallel implementation | 4 agents implement simultaneously |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | Audit agents write findings; dashboard agent reads them |
| Dependency ordering | operator-dashboard waits for all 4 audits |
| Scope enforcement | Verified at integration time |
| Worktree cleanup | All 5 removed at end |
| Heartbeat compliance | Agents themselves must heartbeat, providing live test data for heartbeat-enforcer to analyze retroactively |

## Meta-Observability: Testing the Tester

This plan has a recursive property: the LOOM run executing this evaluation is itself an observability target. After the run completes, an operator should be able to:

1. Run `heartbeat-enforcer`'s script against the evaluation's own loom branches to verify the evaluation agents themselves were heartbeat-compliant.
2. Run `git-trail-auditor`'s script against the evaluation's own branches to verify trailer compliance.
3. Inspect `operator-dashboard`'s report to see whether the evaluation's own observability was sufficient.

This self-referential check is the strongest possible validation: if LOOM's observability mechanisms work, they should be able to observe themselves.

## Features NOT Tested

- BLOCKED/FAILED states (as first-class agent outcomes -- branch-retention simulates them but no agent is expected to actually fail)
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Cross-agent read access to peer STATUS.md (agent-to-agent coordination)
- Level 2 budget tracking fields
- Content-addressed memory refs (`refs/loom/memory/`)

## Expected Findings (Hypotheses)

These are predictions about what the audit will reveal. They guide agent work but do not constrain it.

1. **Heartbeat blind spot**: The 5-minute heartbeat + commit model has a worst-case detection latency of `timeout_seconds` (default 3600s = 1 hour) for an agent that stalls between commits. This is too slow for production use.

2. **STATUS.md fragility**: YAML front matter parsing is sensitive to quoting. Agents that write unquoted strings containing colons or hashes will produce invalid YAML that silently corrupts the status signal.

3. **Git trail is strong**: Commit trailers plus `git log` provide a robust, immutable audit trail. This is LOOM's strongest observability signal.

4. **Branch retention is unenforced**: The 30-day retention policy is stated but has no implementation mechanism. Any user with repo write access can delete a failed branch. There is no warning, no lock, no tag.

5. **No centralized run ID**: There is no single identifier that ties all agents in a LOOM run together. Session-Id is per-agent. Reconstructing which agents belonged to the same run requires tracing merge commits on the workspace branch -- possible but fragile.
