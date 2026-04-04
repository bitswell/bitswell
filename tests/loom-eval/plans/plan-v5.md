# LOOM Evaluation Plan v5 -- Error Recovery Marathon

## Goal

Deliberately trigger all five LOOM error categories (`task_unclear`, `blocked`, `resource_limit`, `conflict`, `internal`) and validate the orchestrator's recovery procedure for each. This plan exercises the failure modes that plan-v0 explicitly left untested: BLOCKED/FAILED states, continuation agents, merge conflict recovery, and heartbeat enforcement.

The real deliverable is a report on whether the protocol's error model is complete, whether the recovery paths actually work, and where gaps exist.

## Error Categories Under Test

| # | Error Category | Agent ID | How Failure Is Simulated | Recovery Path |
|---|----------------|----------|--------------------------|---------------|
| 1 | `task_unclear` | `err-unclear` | TASK.md contains contradictory acceptance criteria | Orchestrator escalates to human; does NOT retry |
| 2 | `blocked` | `err-blocked` | Agent depends on `err-dep` which is deliberately delayed | Orchestrator resolves blocker, transitions agent back to IMPLEMENTING |
| 3 | `resource_limit` | `err-budget` | AGENT.json sets `token_budget` absurdly low (5000 tokens) | Orchestrator spawns a continuation agent from the blocked agent's branch |
| 4 | `conflict` | `err-conflict-a` + `err-conflict-b` | Two agents given overlapping scopes that modify the same file | Orchestrator aborts merge, attempts rebase, falls back to fresh agent |
| 5 | `internal` | `err-internal` | TASK.md instructs agent to run a git command that corrupts its index | Orchestrator preserves worktree for post-mortem, spawns replacement agent |

## Agent Decomposition

| Agent ID | Role | Deps | Scope (`paths_allowed`) | Purpose |
|----------|------|------|-------------------------|---------|
| `err-unclear` | Canary | none | `tests/loom-eval/err-unclear/**` | Trigger `task_unclear` by failing to reconcile contradictions in TASK.md |
| `err-dep` | Dependency stub | none | `tests/loom-eval/err-dep/**` | Provides the prerequisite that `err-blocked` depends on; completes normally but is integrated late |
| `err-blocked` | Blocked agent | `err-dep` | `tests/loom-eval/err-blocked/**` | Enters BLOCKED because its dependency is not yet integrated; tests the unblock path |
| `err-budget` | Budget canary | none | `tests/loom-eval/err-budget/**` | Hits 90% budget almost immediately due to a 5000-token ceiling; tests continuation agent |
| `err-budget-cont` | Continuation | none | `tests/loom-eval/err-budget/**` | Spawned from `err-budget`'s branch to finish the remaining work |
| `err-conflict-a` | Writer A | none | `tests/loom-eval/err-conflict/**` | Writes a file that will conflict with Writer B |
| `err-conflict-b` | Writer B | none | `tests/loom-eval/err-conflict/**` | Writes the same file differently; second to integrate, triggers merge conflict |
| `err-conflict-b-redo` | Conflict recovery | none | `tests/loom-eval/err-conflict/**` | Fresh agent spawned from workspace HEAD after conflict; resolves the conflicting file |
| `err-internal` | Crash agent | none | `tests/loom-eval/err-internal/**` | Encounters an internal error (simulated git corruption); tests post-mortem path |
| `err-internal-redo` | Replacement | none | `tests/loom-eval/err-internal/**` | Replacement agent spawned from workspace HEAD with context from the crashed agent's MEMORY.md |
| `eval-marathon` | Reporter | all above | `tests/loom-eval/report/**` | Reads MEMORY.md and STATUS.md from every agent; compiles the marathon report |

No scope overlap between error categories (each writes to its own subdirectory). The deliberate conflict between `err-conflict-a` and `err-conflict-b` is the exception -- they share `tests/loom-eval/err-conflict/**` by design.

## Failure Simulation Details

### 1. `task_unclear` -- Agent `err-unclear`

**TASK.md design:** The acceptance criteria contain an irreconcilable contradiction:

```markdown
## Acceptance Criteria
- [ ] Output MUST be a single JSON file with no nested objects
- [ ] Output MUST preserve the full tree structure from the input, including all nested objects
```

These two criteria cannot both be satisfied. A well-behaved LOOM worker should:
1. Detect the contradiction during PLANNING.
2. Set STATUS.md to `FAILED` with `error.category: task_unclear`.
3. Write MEMORY.md explaining which criteria conflict and why.
4. Exit with code 1.

**Orchestrator recovery:**
1. Read STATUS.md. Verify `status: FAILED` and `error.category: task_unclear`.
2. Read MEMORY.md. Verify the agent identified the contradiction.
3. Per protocol Section 6.3: escalate to human. Do NOT retry automatically.
4. Log the escalation. Record in the marathon report that the protocol was followed.
5. Retain the `loom/err-unclear` branch (never delete failed branches).

**Pass criteria:** Agent enters FAILED with the correct error category. Orchestrator does not retry. Branch is retained.

### 2. `blocked` -- Agent `err-blocked` (depends on `err-dep`)

**Simulation:** Both agents are spawned for planning in parallel. After plan approval, `err-dep` is spawned for implementation first but the orchestrator deliberately delays its integration. Meanwhile, `err-blocked` is spawned for implementation, discovers its dependency is not integrated, and enters BLOCKED.

**TASK.md for `err-blocked`:**

```markdown
## Objective
Read the output from err-dep (at tests/loom-eval/err-dep/output.json) and
transform it. If the dependency output does not exist, you are blocked.

## Dependencies
- err-dep
```

**Agent behavior:**
1. During IMPLEMENTING, the agent checks for the dependency output.
2. Finding it absent, it sets STATUS.md to `BLOCKED` with `blocked_reason: "Dependency err-dep not yet integrated; required file tests/loom-eval/err-dep/output.json does not exist"`.
3. Commits and exits with code 1.

**Orchestrator recovery:**
1. Read STATUS.md. Confirm `status: BLOCKED`.
2. Now integrate `err-dep` into the workspace.
3. Update the `err-blocked` worktree: `git -C .worktrees/err-blocked merge HEAD`.
4. Append `## Feedback` to TASK.md: `"Dependency err-dep is now integrated. output.json is available. Resume implementation."`
5. Re-spawn `err-blocked` for implementation. The agent should transition BLOCKED -> IMPLEMENTING -> COMPLETED.

**Pass criteria:** Agent correctly enters BLOCKED (not FAILED). Orchestrator unblocks by integrating the dependency and re-spawning. Agent completes on the second spawn.

### 3. `resource_limit` -- Agent `err-budget`

**Simulation:** AGENT.json sets `token_budget: 5000`. The task requires substantially more work than 5000 tokens can accomplish (e.g., "analyze all 5 LOOM reference files and produce a summary").

**Agent behavior (per protocol Section 10.3):**
1. Agent begins work but detects it has consumed ~90% of its 5000-token budget almost immediately.
2. Writes MEMORY.md with progress so far and a clear description of remaining work.
3. Sets STATUS.md to `BLOCKED` with `blocked_reason: resource_limit`.
4. Includes `budget: { tokens_used: ~4500, tokens_limit: 5000 }`.
5. Commits and exits with code 1.

**Orchestrator recovery (per SKILL.md "Error: Resource Limit"):**
1. Read MEMORY.md from `.worktrees/err-budget/MEMORY.md`.
2. Note what was completed and what remains.
3. Spawn a continuation agent branching from the blocked agent's branch:
   ```bash
   git branch loom/err-budget-cont loom/err-budget
   git worktree add .worktrees/err-budget-cont loom/err-budget-cont
   ```
4. Write a new TASK.md covering only the remaining work. Reference findings from the prior MEMORY.md.
5. Set a larger `token_budget` in AGENT.json for the continuation agent (e.g., 100000).
6. Run the standard two-phase cycle (plan, gate, implement) on `err-budget-cont`.
7. Integrate `err-budget-cont` (not the original `err-budget`).

**Pass criteria:** Original agent checkpoints correctly at 90% budget. Continuation agent picks up where the original left off. The combined work of both agents produces a complete result.

### 4. `conflict` -- Agents `err-conflict-a` and `err-conflict-b`

**Simulation:** Both agents are given the same scope (`tests/loom-eval/err-conflict/**`) and are tasked with modifying the same file (`tests/loom-eval/err-conflict/shared.md`). They are independent (no declared dependency), so they plan and implement in parallel.

**TASK.md for `err-conflict-a`:**
```markdown
## Objective
Create tests/loom-eval/err-conflict/shared.md with a section titled
"## Analysis A" containing your findings about LOOM error handling.
```

**TASK.md for `err-conflict-b`:**
```markdown
## Objective
Create tests/loom-eval/err-conflict/shared.md with a section titled
"## Analysis B" containing your findings about LOOM state transitions.
```

Both agents will create `shared.md` with different content. The first integration succeeds; the second triggers a merge conflict.

**Orchestrator recovery (per SKILL.md "Error: Merge Conflict"):**
1. Integrate `err-conflict-a` first. Merge succeeds.
2. Attempt to integrate `err-conflict-b`. Merge fails with conflict in `shared.md`.
3. Immediately abort: `git merge --abort`.
4. **Recovery Option A -- rebase:**
   ```bash
   git branch loom/err-conflict-b-v2 loom/err-conflict-b
   git rebase HEAD loom/err-conflict-b-v2
   ```
   This will also conflict since both create the same file. Abort the rebase.
5. **Recovery Option B -- fresh agent (`err-conflict-b-redo`):**
   ```bash
   git branch loom/err-conflict-b-redo HEAD
   git worktree add .worktrees/err-conflict-b-redo loom/err-conflict-b-redo
   ```
   Write a new TASK.md that says: "The file `shared.md` already exists with Analysis A. Append an `## Analysis B` section without removing the existing content. See the failed agent's MEMORY.md for context."
   Include the failed agent's MEMORY.md findings in the new TASK.md context.
6. Run two-phase cycle on `err-conflict-b-redo`. Integrate cleanly.
7. Retain `loom/err-conflict-b` (never delete failed branches).

**Pass criteria:** Orchestrator correctly aborts the failed merge (workspace stays clean). Rebase is attempted. Fresh agent is spawned when rebase fails. Final integrated file contains both Analysis A and Analysis B. Failed branch is retained.

### 5. `internal` -- Agent `err-internal`

**Simulation:** TASK.md contains a subtask that causes a git operation to fail in an unexpected way. For example, the task instructs the agent to work with a file whose path contains an invalid character sequence, or the orchestrator pre-corrupts a file in the worktree (e.g., places a broken symlink or a binary file where a text file is expected).

Practically, the simplest simulation: before spawning the implementation phase, the orchestrator places a file `.git/index.lock` in the agent's worktree git dir, causing all git operations to fail with "Another git process seems to be running." The agent cannot commit its work.

**Agent behavior:**
1. Agent begins implementation, attempts to commit, hits the lock error.
2. If the agent can still write files (even if it cannot commit), it writes MEMORY.md and STATUS.md with `status: FAILED`, `error.category: internal`, `error.message: "git index locked -- unable to commit"`.
3. If the agent cannot write at all, it exits with code 2 (catastrophic).

**Orchestrator recovery (per protocol Section 6.3):**
1. Read STATUS.md (if it was written) or inspect the worktree directly (exit code 2 case).
2. Preserve the worktree for post-mortem analysis. Do NOT delete it or its branch.
3. Clean up the simulated corruption (remove the lock file) for the post-mortem record.
4. Spawn a replacement agent `err-internal-redo` from workspace HEAD:
   ```bash
   git branch loom/err-internal-redo HEAD
   git worktree add .worktrees/err-internal-redo loom/err-internal-redo
   ```
5. Write TASK.md for the replacement with context from the failed agent's MEMORY.md (if available).
6. Run two-phase cycle. Integrate.

**Pass criteria:** Agent correctly identifies the error as `internal`. Orchestrator preserves the failed worktree. Replacement agent completes the work. Failed branch `loom/err-internal` is retained.

## Execution Flow

```
Phase 0: Setup
  Create seed file for the conflict test:
    (No seed file needed -- both agents create shared.md, guaranteeing a conflict.)

Phase 1: Create all worktrees (8 initial agents)
  git worktree add .worktrees/err-unclear      -b loom/err-unclear
  git worktree add .worktrees/err-dep           -b loom/err-dep
  git worktree add .worktrees/err-blocked       -b loom/err-blocked
  git worktree add .worktrees/err-budget        -b loom/err-budget
  git worktree add .worktrees/err-conflict-a    -b loom/err-conflict-a
  git worktree add .worktrees/err-conflict-b    -b loom/err-conflict-b
  git worktree add .worktrees/err-internal      -b loom/err-internal
  git worktree add .worktrees/eval-marathon     -b loom/eval-marathon
  Write TASK.md + AGENT.json for each. Commit.

Phase 2: PLANNING -- spawn all 7 working agents in parallel
  (eval-marathon is not spawned yet -- it depends on all others.)
  err-unclear, err-dep, err-blocked, err-budget,
  err-conflict-a, err-conflict-b, err-internal all write PLAN.md.

Phase 3: PLAN GATE
  Read all 7 PLAN.md files.
  Expected: err-unclear may FAIL during planning (contradictory task).
  If it plans successfully, approve it -- the failure should occur during
  implementation when the contradiction becomes irreconcilable.
  Approve all others. Check scope: only err-conflict-a and err-conflict-b
  share scope (intentional).

Phase 4: IMPLEMENTATION -- first wave (parallel)
  Spawn: err-unclear, err-dep, err-budget, err-conflict-a, err-conflict-b, err-internal.
  Do NOT spawn err-blocked yet (its dependency is not integrated).
  Before spawning err-internal: inject the simulated corruption
    (touch .worktrees/err-internal/.git/index.lock or equivalent).

Phase 5: Harvest first wave results
  Expected outcomes:
    err-unclear    -> FAILED (task_unclear)
    err-dep        -> COMPLETED
    err-budget     -> BLOCKED (resource_limit)
    err-conflict-a -> COMPLETED
    err-conflict-b -> COMPLETED
    err-internal   -> FAILED (internal)

Phase 6: Recovery -- task_unclear (err-unclear)
  1. Read STATUS.md -- verify FAILED, error.category = task_unclear.
  2. Read MEMORY.md -- verify contradiction is documented.
  3. Log escalation. Do NOT retry.
  4. Record result in marathon scorecard.

Phase 7: Recovery -- resource_limit (err-budget -> err-budget-cont)
  1. Read MEMORY.md from err-budget.
  2. Create continuation:
       git branch loom/err-budget-cont loom/err-budget
       git worktree add .worktrees/err-budget-cont loom/err-budget-cont
  3. Write new TASK.md (remaining work only) + AGENT.json (larger budget).
  4. Plan phase -> gate -> implement phase on err-budget-cont.
  5. Integrate err-budget-cont.

Phase 8: Integration -- err-dep and err-conflict-a
  1. Integrate err-dep: git merge --no-ff loom/err-dep
  2. Integrate err-conflict-a: git merge --no-ff loom/err-conflict-a

Phase 9: Recovery -- blocked (err-blocked)
  1. err-dep is now integrated.
  2. Update err-blocked worktree: git -C .worktrees/err-blocked merge HEAD
  3. Append Feedback to TASK.md. Commit.
  4. Re-spawn err-blocked for implementation.
  5. On COMPLETED: integrate err-blocked.

Phase 10: Recovery -- conflict (err-conflict-b -> err-conflict-b-redo)
  1. Attempt: git merge --no-ff loom/err-conflict-b  (expect CONFLICT)
  2. git merge --abort
  3. Attempt rebase: git rebase HEAD loom/err-conflict-b  (expect conflict)
  4. git rebase --abort
  5. Spawn fresh agent err-conflict-b-redo from workspace HEAD.
  6. Two-phase cycle on err-conflict-b-redo.
  7. Integrate err-conflict-b-redo.
  8. Retain loom/err-conflict-b.

Phase 11: Recovery -- internal (err-internal -> err-internal-redo)
  1. Inspect .worktrees/err-internal -- read STATUS.md and MEMORY.md if they exist.
  2. Remove the simulated corruption (cleanup for the record).
  3. Spawn replacement err-internal-redo from workspace HEAD.
  4. Two-phase cycle. Integrate.
  5. Retain loom/err-internal.

Phase 12: Integration -- err-budget-cont (if not already done in Phase 7)

Phase 13: Reporter agent -- eval-marathon
  1. All error agents are now resolved (completed, failed-and-escalated, or replaced).
  2. Spawn eval-marathon for planning. It reads MEMORY.md and STATUS.md from
     every agent worktree.
  3. Plan gate. Approve.
  4. Spawn eval-marathon for implementation. It compiles the marathon report.
  5. Integrate eval-marathon.

Phase 14: Cleanup
  Remove all worktrees.
  Retain all failed branches (loom/err-unclear, loom/err-conflict-b, loom/err-internal)
  for 30 days per protocol Section 6.3.
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 11 agents total, each in its own worktree |
| Parallel planning | 7 agents plan simultaneously |
| Plan gate | Orchestrator reviews 7 plans; deliberately approves conflicting scopes |
| Parallel implementation | 6 agents implement simultaneously in first wave |
| STATUS.md lifecycle: PLANNING | All agents pass through PLANNING |
| STATUS.md lifecycle: IMPLEMENTING | Normal agents and recovered agents |
| STATUS.md lifecycle: COMPLETED | err-dep, err-conflict-a, err-conflict-b, continuation agents, reporter |
| STATUS.md lifecycle: BLOCKED | err-blocked (dependency), err-budget (resource limit) |
| STATUS.md lifecycle: FAILED | err-unclear (task_unclear), err-internal (internal) |
| Error category: `task_unclear` | err-unclear -- contradictory acceptance criteria |
| Error category: `blocked` | err-blocked -- unmet dependency |
| Error category: `resource_limit` | err-budget -- absurdly small token budget |
| Error category: `conflict` | err-conflict-b -- merge conflict at integration |
| Error category: `internal` | err-internal -- simulated git corruption |
| Recovery: escalate to human | err-unclear -- orchestrator logs and does not retry |
| Recovery: unblock and re-spawn | err-blocked -- dependency integrated, agent re-spawned |
| Recovery: continuation agent | err-budget-cont -- branches from blocked agent, finishes remaining work |
| Recovery: merge --abort + rebase | err-conflict-b -- abort, rebase attempt, fresh agent fallback |
| Recovery: post-mortem preservation | err-internal -- worktree and branch preserved, replacement spawned |
| Commit trailers | Every agent commit verified for Agent-Id + Session-Id |
| MEMORY.md handoff | Budget agent -> continuation agent; failed agents -> replacement agents; all -> reporter |
| Dependency ordering | err-blocked waits for err-dep integration |
| Scope enforcement | Verified at integration; conflict agents share scope intentionally |
| Branch retention | Failed branches (err-unclear, err-conflict-b, err-internal) never deleted |
| Worktree cleanup | All worktrees removed at end except those retained for post-mortem |
| Heartbeat liveness | err-internal may fail to heartbeat -- orchestrator should detect staleness |

## Marathon Scorecard

The `eval-marathon` reporter agent produces a scorecard with one row per error category:

| Error Category | Agent | Failed Correctly? | Recovery Followed Protocol? | Final Outcome | Notes |
|---|---|---|---|---|---|
| `task_unclear` | err-unclear | Y/N | Y/N (escalated, no retry) | Escalated / Wrong | |
| `blocked` | err-blocked | Y/N | Y/N (unblocked, re-spawned) | Completed / Stuck | |
| `resource_limit` | err-budget | Y/N | Y/N (continuation agent) | Completed via cont / Stuck | |
| `conflict` | err-conflict-b | Y/N | Y/N (abort, rebase, fresh agent) | Completed via redo / Stuck | |
| `internal` | err-internal | Y/N | Y/N (preserved, replacement) | Completed via redo / Stuck | |

**Overall pass:** All 5 rows show "Failed Correctly = Y" AND "Recovery Followed Protocol = Y" AND the final outcome is correct.

## Features NOT Tested

- Cycle detection in dependency DAG (no cycles are introduced)
- Concurrent agent limit enforcement (max_agents = 10; we use 11 total but never more than 7 simultaneously)
- Heartbeat timeout termination (SIGTERM/SIGKILL sequence)
- Level 2 features (content-addressed memory refs, budget tracking beyond Level 1)
- Multiple rounds of plan feedback (all plans are approved on first pass or agents fail during planning)
- Nested continuation chains (only one continuation depth: err-budget -> err-budget-cont)
