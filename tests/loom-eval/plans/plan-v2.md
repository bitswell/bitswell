# LOOM Evaluation Plan v2 -- Minimal Viable Test

## Goal

Prove the LOOM happy path works end-to-end with the absolute minimum complexity: 1 agent, 1 trivial task, 0 dependencies. If this fails, nothing more complex can succeed. If it passes, the core machinery (worktree isolation, two-phase lifecycle, plan gate, commit trailers, scope enforcement, integration merge, cleanup) is validated.

## Task

The agent creates a single file: `tests/loom-eval/hello/hello.txt` containing the text `hello from loom`. That is the entire deliverable.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|-------------|
| `hello` | Writer | Create `tests/loom-eval/hello/hello.txt` with specified content | none |

## Scope

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `hello` | `tests/loom-eval/hello/**` | `[]` |

No scope overlap possible (single agent).

## Execution Flow -- Exact Commands

Every orchestrator command is listed below. Nothing is implied. The orchestrator session ID is represented as `$ORCH_SID` and the worker session ID as `$WORKER_SID`; generate both with `uuidgen` before starting.

### Step 0: Generate session IDs

```bash
ORCH_SID=$(python3 -c "import uuid; print(uuid.uuid4())")
WORKER_SID=$(python3 -c "import uuid; print(uuid.uuid4())")
echo "Orchestrator SID: $ORCH_SID"
echo "Worker SID: $WORKER_SID"
```

Record `BASE_COMMIT` for use in STATUS.md:

```bash
BASE_COMMIT=$(git rev-parse HEAD)
echo "Base commit: $BASE_COMMIT"
```

### Step 1: Create worktree and branch

```bash
git worktree add .worktrees/hello -b loom/hello
```

### Step 2: Write TASK.md into the worktree

Write `.worktrees/hello/TASK.md`:

```markdown
# Task: Create hello.txt

## Objective
Create a single file `tests/loom-eval/hello/hello.txt` containing the text `hello from loom`.

## Context
This is a minimal LOOM evaluation. The purpose is to validate that the protocol machinery works end-to-end. The task is intentionally trivial.

## Scope
- **Allowed paths**: `tests/loom-eval/hello/**`
- **Denied paths**: (none)

## Acceptance Criteria
- [ ] File `tests/loom-eval/hello/hello.txt` exists
- [ ] File contains exactly `hello from loom` (with trailing newline)
- [ ] No other files are created or modified (outside protocol files)

## Dependencies
- none

## Constraints
- **Token budget**: 50000
- **Timeout**: 600
```

### Step 3: Write AGENT.json into the worktree

Write `.worktrees/hello/AGENT.json`:

```json
{
  "agent_id": "hello",
  "session_id": "<$WORKER_SID>",
  "protocol_version": "loom/1",
  "context_window_tokens": 200000,
  "token_budget": 50000,
  "dependencies": [],
  "scope": {
    "paths_allowed": ["tests/loom-eval/hello/**"],
    "paths_denied": []
  },
  "timeout_seconds": 600
}
```

### Step 4: Commit the assignment

```bash
git -C .worktrees/hello add TASK.md AGENT.json
git -C .worktrees/hello commit -m "$(cat <<EOF
chore(loom): assign hello

Agent-Id: orchestrator
Session-Id: $ORCH_SID
EOF
)"
```

### Step 5: PLANNING PHASE -- spawn the worker

Build the Agent tool prompt by reading `references/worker-template.md` and substituting:
- `{{WORKTREE_PATH}}` with the absolute path to `.worktrees/hello`
- `{{AGENT_ID}}` with `hello`
- `{{SESSION_ID}}` with `$WORKER_SID`

Append to the filled template:

> This is your PLANNING phase. Read TASK.md and AGENT.json. Write PLAN.md. Update STATUS.md to PLANNING. Commit both. Then return. Do NOT implement.

Spawn the agent via the Agent tool. Wait for it to return.

### Step 6: PLAN GATE -- review the plan

Read the plan and status:

```bash
cat .worktrees/hello/PLAN.md
head -20 .worktrees/hello/STATUS.md
```

**Checks (all must pass):**

| # | Check | Command |
|---|-------|---------|
| 1 | PLAN.md exists and has all 5 required sections (Approach, Steps, Files to Modify, Risks, Estimated Effort) | `grep -c '^## ' .worktrees/hello/PLAN.md` -- expect >= 5 |
| 2 | STATUS.md has valid YAML front matter with `status: PLANNING` | `head -20 .worktrees/hello/STATUS.md` |
| 3 | Commit has Agent-Id trailer | `git -C .worktrees/hello log -1 --format=%B \| grep 'Agent-Id: hello'` |
| 4 | Commit has Session-Id trailer | `git -C .worktrees/hello log -1 --format=%B \| grep 'Session-Id:'` |
| 5 | Only `tests/loom-eval/hello/**` appears in planned file modifications | Visual inspection of PLAN.md |

If any check fails, append `## Feedback` to TASK.md with corrections, commit, and re-spawn planning. For this trivial task, re-planning should not be needed.

Approve the plan:

```bash
cd .worktrees/hello
echo -e "\n## Feedback\n\nApproved. Proceed with implementation." >> TASK.md
git add TASK.md
git commit -m "$(cat <<EOF
chore(loom): approve hello plan

Agent-Id: orchestrator
Session-Id: $ORCH_SID
EOF
)"
```

### Step 7: IMPLEMENTATION PHASE -- re-spawn the worker

Re-read `references/worker-template.md`, substitute the same placeholders, and append:

> This is your IMPLEMENTATION phase. Your plan was approved. Read PLAN.md, implement the work, write MEMORY.md, set STATUS.md to COMPLETED, commit, and return.

Spawn the agent via the Agent tool. Wait for it to return.

### Step 8: Validate the agent's work

```bash
# Check status
head -20 .worktrees/hello/STATUS.md
```

**Checks (all must pass):**

| # | Check | Command |
|---|-------|---------|
| 1 | STATUS.md says `status: COMPLETED` | `grep 'status: COMPLETED' .worktrees/hello/STATUS.md` |
| 2 | `files_changed` field is present | `grep 'files_changed:' .worktrees/hello/STATUS.md` |
| 3 | MEMORY.md exists with all 3 required sections | `grep -c '^## ' .worktrees/hello/MEMORY.md` -- expect >= 3 |
| 4 | The deliverable file exists | `cat .worktrees/hello/tests/loom-eval/hello/hello.txt` |
| 5 | The deliverable file has the correct content | Content is exactly `hello from loom` |
| 6 | No files outside scope were modified | `git -C .worktrees/hello diff --name-only $BASE_COMMIT -- ':!TASK.md' ':!AGENT.json' ':!PLAN.md' ':!STATUS.md' ':!MEMORY.md'` -- only `tests/loom-eval/hello/hello.txt` should appear |
| 7 | All commits have Agent-Id trailer | `git -C .worktrees/hello log --format=%B $BASE_COMMIT..HEAD \| grep -c 'Agent-Id:'` matches commit count |
| 8 | All commits have Session-Id trailer | Same pattern with `Session-Id:` |

### Step 9: Integrate into workspace

```bash
git merge --no-ff loom/hello -m "$(cat <<EOF
feat(loom): integrate hello

Agent-Id: orchestrator
Session-Id: $ORCH_SID
EOF
)"
```

**Post-integration check:**

```bash
# Verify the file landed in the workspace
cat tests/loom-eval/hello/hello.txt
# Should output: hello from loom
```

### Step 10: Clean up

```bash
git worktree remove .worktrees/hello
```

Verify cleanup:

```bash
git worktree list   # Should only show the main workspace
ls .worktrees/      # Should be empty or not exist
```

## LOOM Features Exercised

| Feature | How | Verdict criteria |
|---------|-----|------------------|
| Worktree isolation | Agent works only in `.worktrees/hello` | No files outside scope modified |
| Two-phase lifecycle | Planning spawn, then implementation spawn | Both phases complete without error |
| Plan gate | Orchestrator reads and approves PLAN.md between phases | PLAN.md has all 5 sections, plan is sane |
| Commit trailers | `Agent-Id` and `Session-Id` on every agent commit | All commits pass trailer check |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED | STATUS.md shows COMPLETED at end |
| MEMORY.md | Agent writes findings with 3 required sections | MEMORY.md exists and has sections |
| Scope enforcement | Agent only writes to `tests/loom-eval/hello/**` | Diff shows no out-of-scope files |
| Integration merge | `--no-ff` merge into workspace | File appears in workspace after merge |
| Worktree cleanup | `git worktree remove` | Worktree gone, branch retained |

## Features NOT Tested

- Multiple agents
- Parallel spawning
- Dependencies between agents
- BLOCKED / FAILED states
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement (task completes too fast to need one)
- Plan rejection / feedback loop
- Scope denial (`paths_denied` is empty)
- Budget tracking (Level 2)

## Why This Plan Exists

Plan v0 tests breadth: 4 agents, dependencies, parallelism, cross-agent data flow. This plan tests depth of the single-agent path. It provides a baseline: if a single agent creating a single file does not work, multi-agent coordination is moot. The trivial task means any failure is attributable to protocol machinery rather than task complexity.

## Pass / Fail

**PASS** if all checks in Steps 6, 8, 9, and 10 succeed.
**FAIL** if any check fails. Record which check failed and the exact error output.
