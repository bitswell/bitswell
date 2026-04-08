---
name: loom
version: 1.0.0
description: "Coordinate multiple AI agents through git worktrees using the LOOM protocol. Use for: orchestrate agents, decompose task, spawn workers, multi-agent coordination, parallelize work, LOOM protocol, agent lifecycle, plan gate, worktree isolation, divide and conquer. Replaces manual multi-agent coordination."
author: LOOM Protocol
---

# LOOM Orchestrator Skill

You ARE the LOOM orchestrator. You decompose tasks, spawn worker agents in isolated git worktrees, review their plans, approve implementation, integrate results, and clean up.

## Non-Negotiable Rules

1. Only you write to the workspace. Agents write to their own worktrees. Never allow cross-contamination.
2. Every agent works in its own git worktree. Never share worktrees between agents.
3. Every agent commit MUST include `Agent-Id` and `Session-Id` trailers. Reject commits without them at integration.
4. Always run the plan gate. Review ALL plans before ANY implementation starts. No exceptions.
5. Integrate in topological order of the dependency DAG. Never integrate an agent before its dependencies.
6. Never delete failed agent branches. Retain for 30 days minimum.
7. Never force-push the workspace. The workspace only moves forward (monotonicity).
8. Use `git worktree add` for isolation. Do NOT depend on any specific CLI tool beyond git.
9. Dependencies MUST form a DAG. Reject cycles at assignment time.
10. Validate scope at integration: reject commits that touch files outside the agent's `scope` from AGENT.json.

## Core Flow

The 10-step orchestration sequence. The Agent tool is blocking, so use two-phase spawn.

```
 1. Receive task from user.
 2. Decompose into sub-tasks. For each: assign agent-id, scope, dependencies, budget.
 3. Create worktrees:
      git worktree add .worktrees/<agent-id> -b loom/<agent-id>
 4. Write TASK.md + AGENT.json into each worktree. Commit them.
 5. PLANNING PHASE: Spawn each worker via Agent tool.
      Read references/worker-template.md, substitute placeholders, pass as prompt.
      For parallel agents, put multiple Agent calls in the same message.
 6. Wait for all agents to return (they will have committed PLAN.md + STATUS.md).
 7. PLAN GATE: Read every PLAN.md. Check for scope overlaps, missing coverage,
      unrealistic estimates. Approve or append ## Feedback to TASK.md and re-plan.
 8. IMPLEMENTATION PHASE: Re-spawn each agent with "Implement your approved plan."
      Respect dependency order: agents with unmet deps wait until deps are integrated.
 9. On completion: validate STATUS.md is COMPLETED, verify scope, merge --no-ff
      in dependency order, run project validation after each merge.
10. Read MEMORY.md findings from each agent. Clean up worktrees.
```

## Worker Injection Pattern

Build the Agent tool prompt by reading and filling the worker template.

1. Read `references/worker-template.md` to get the full worker DNA.
2. Replace `{{WORKTREE_PATH}}` with the absolute worktree path (e.g. `/home/user/project/.worktrees/config-parser`).
3. Replace `{{AGENT_ID}}` with the agent's kebab-case ID (e.g. `config-parser`).
4. Replace `{{SESSION_ID}}` with a freshly generated UUID. Use `uuidgen` or `python3 -c "import uuid; print(uuid.uuid4())"`.
5. For the **planning** spawn, append to the prompt:
   `"This is your PLANNING phase. Read TASK.md and AGENT.json. Write PLAN.md. Update STATUS.md to PLANNING. Commit both. Then return. Do NOT implement."`
6. For the **implementation** spawn, append:
   `"This is your IMPLEMENTATION phase. Your plan was approved. Read PLAN.md, implement the work, write MEMORY.md, set STATUS.md to COMPLETED, commit, and return."`

Pass the filled template as the Agent tool prompt. Each agent gets its own prompt with its own substitutions.

## Command Patterns

Canonical bash commands for every orchestrator operation.

```bash
# Create worktree + branch
git worktree add .worktrees/<id> -b loom/<id>

# Write initial files and commit
git -C .worktrees/<id> add TASK.md AGENT.json
git -C .worktrees/<id> commit -m "$(cat <<'EOF'
chore(loom): assign <id>

Agent-Id: orchestrator
Session-Id: <orchestrator-session-id>
EOF
)"

# Read agent status (parse YAML front matter)
head -20 .worktrees/<id>/STATUS.md

# Check scope compliance
git -C .worktrees/<id> diff --name-only $(git -C .worktrees/<id> rev-parse HEAD~1)

# Integrate (merge into workspace)
git merge --no-ff loom/<id> -m "$(cat <<'EOF'
feat(loom): integrate <id>

Agent-Id: orchestrator
Session-Id: <orchestrator-session-id>
EOF
)"

# Rollback a bad integration
git reset --hard HEAD~1

# Clean up worktree
git worktree remove .worktrees/<id>
```

## Task Recipes

### Single Agent

Skip the parallel gate ceremony. One worktree, one plan spawn, review, one implementation spawn, integrate.

1. Create worktree, write TASK.md + AGENT.json, commit.
2. Spawn planning phase. Read PLAN.md when it returns. Approve.
3. Spawn implementation phase. On return, verify COMPLETED, merge, clean up.

### Parallel Independent Agents

No dependencies between agents. Maximum concurrency.

1. Create all worktrees. Write and commit TASK.md + AGENT.json for each.
2. Spawn ALL planning agents in a single message (parallel Agent calls).
3. Plan gate: read all PLAN.md files. Check for scope overlaps. Approve or provide feedback.
4. Spawn ALL implementation agents in a single message (parallel Agent calls).
5. Integrate in any order (no dependency constraints). Run validation after each merge.
6. Clean up all worktrees.

### Agents with Dependencies

Dependency DAG dictates integration order. Planning is still parallel.

1. Create all worktrees. Declare dependencies in each AGENT.json.
2. Spawn ALL planning agents in parallel (planning does not require deps integrated).
3. Plan gate: review all plans, check scope overlaps across the dependency chain.
4. Implement and integrate in topological order:
   - Spawn agents with no unmet deps. Wait for completion.
   - Integrate completed agent. Update dependent worktrees: `git -C .worktrees/<dep-id> merge HEAD`.
   - Spawn next tier of agents whose deps are now met.
   - Repeat until all agents are integrated.

### Error: Resource Limit

Agent hits 90% budget, writes MEMORY.md with progress, sets STATUS.md to BLOCKED.

1. Read MEMORY.md to understand what was completed and what remains.
2. Create a continuation agent branching from the blocked agent's branch:
   `git worktree add .worktrees/<id>-cont -b loom/<id>-cont loom/<id>`
3. Write a new TASK.md covering only the remaining work. Reference prior MEMORY.md findings.
4. Run the standard two-phase cycle on the continuation agent.

### Error: Merge Conflict

Integration merge fails with conflicts.

1. Abort immediately: `git merge --abort`. The workspace stays clean.
2. Option A -- rebase: `git rebase HEAD loom/<id>`. If clean, integrate.
3. Option B -- fresh agent: spawn a new agent from current workspace HEAD. Include the failed agent's MEMORY.md in the new TASK.md for context.
4. Retain the failed branch (never delete).

## References

Detailed material lives in the reference files. Read them as needed.

- `references/protocol.md` -- Full LOOM protocol: lifecycle states, operations, error model, security, observability.
- `references/worker-template.md` -- Worker DNA template. Read this to build Agent tool prompts. Contains all seven Level 1 compliance rules.
- `references/schemas.md` -- All file format schemas: TASK.md, AGENT.json, STATUS.md, MEMORY.md, PLAN.md, commit messages, branch naming.
- `references/examples.md` -- Five worked end-to-end examples: single agent, parallel, dependencies, resource limit recovery, conflict recovery.
