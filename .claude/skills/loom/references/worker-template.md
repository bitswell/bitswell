# LOOM Worker Template

**Version**: 2.0.0-draft | **Protocol**: `loom/2` | **Status**: Draft

A worker agent operates exclusively through git commits. No protocol files exist
in the worktree. Task comes from the ASSIGNED commit. State lives in trailers.

---

## 1. Startup Sequence

When spawned, the agent:

1. Reads its identity: `agents/<name>/identity.md`
2. Reads its config: AGENT.json at `.mcagent/agents/<name>/<assignment>/AGENT.json`
3. Reads its task: `git log -1 --format='%B' HEAD` — the ASSIGNED commit message
   contains the full task description, scope, budget, and dependencies.
4. Makes a first commit with `Task-Status: IMPLEMENTING` (no file changes required).

**Critical**: The agent MUST NOT read TASK.md, PLAN.md, STATUS.md, or MEMORY.md.
These files do not exist. The commit log is the only protocol interface.

---

## 2. During Work

The agent works exclusively within its worktree (its PWD). It MUST:

- Commit deliverable changes with `Agent-Id`, `Session-Id`, and `Heartbeat` trailers.
- Commit at least every 5 minutes (a heartbeat-only commit with no file changes is valid).
- Stay within `scope.paths_allowed` from AGENT.json.
- Reserve >=10% of `token_budget` for the final commit.

At 90% budget consumption, commit `Task-Status: BLOCKED` with
`Blocked-Reason: resource_limit` and exit.

---

## 3. Commit Format

### First commit (IMPLEMENTING)

```
chore(<scope>): begin <assignment-slug>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: IMPLEMENTING
Heartbeat: <ISO-8601 UTC>
```

### Work commits (no Task-Status required)

```
<type>(<scope>): <subject>

<body explaining why, not what>

Agent-Id: <agent-id>
Session-Id: <session-id>
Heartbeat: <ISO-8601 UTC>
```

### Final commit (COMPLETED)

```
<type>(<scope>): <subject>

<summary of what was accomplished>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: COMPLETED
Files-Changed: <integer>
Key-Finding: <most important discovery>
Heartbeat: <ISO-8601 UTC>
```

### Blocked commit

```
chore(<scope>): blocked -- <short reason>

<detailed explanation>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: BLOCKED
Blocked-Reason: <description>
Heartbeat: <ISO-8601 UTC>
```

### Failed commit

```
chore(<scope>): failed -- <short reason>

<detailed explanation>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: FAILED
Error-Category: <task_unclear|blocked|resource_limit|conflict|internal>
Error-Retryable: <true|false>
```

---

## 4. Reading Protocol State

The agent queries its own history using `git log --format`:

```bash
# What is my current task?
git log -1 --format='%B' HEAD

# What have I already found?
git log --format='%(trailers:key=Key-Finding,valueonly)' | grep -v '^$'

# What decisions have I made?
git log --format='%(trailers:key=Decision,valueonly)' | grep -v '^$'

# What is my assignment ID?
git log --format='%(trailers:key=Assignment,valueonly)' \
  --grep='Task-Status: ASSIGNED' | head -1
```

---

## 5. What the Worktree Contains

The worktree is a normal git checkout. `git status` shows only deliverable changes.

**Never present**: TASK.md, PLAN.md, STATUS.md, MEMORY.md, AGENT.json.

These artifacts do not belong to the codebase. They either live in
`.mcagent/agents/<name>/<assignment>/` (AGENT.json) or in commit messages
(all protocol state).

---

*End of LOOM Worker Template v2.0.0-draft.*
