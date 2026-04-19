# Agent Teams + LOOM hybrid protocol

Two orchestration models run in this repo: the **LOOM protocol** (git-native, strict, audit-first) and **Claude Code's Agent Teams** (runtime coordination via `TeamCreate` + `SendMessage`). Each has strengths the other lacks. This doc names the hybrid we use: LOOM as the protocol layer (what gets written to git), Teams as the carrier (how agents talk while working).

## The models

| Axis | LOOM | Agent Teams |
|---|---|---|
| Layer | Protocol on git — commits, trailers, scope, DAG | Runtime — task list, messages, peer DMs |
| Agent lifecycle | Blocking two-phase spawn (plan, gate, implement) | Persistent teammates; idle after each turn |
| Coordination | None mid-work; orchestrator reviews output | Real-time DMs (lead ↔ teammate, teammate ↔ teammate) |
| Invariants | Scope-enforced at merge, DAG-validated, monotonic workspace | Task-list ownership, shutdown protocol |
| Weakness | No mid-work signaling; workers can't ask each other questions | No scope contract; no built-in audit trail |

The hybrid takes what each does best.

## Layout — everything in git, nothing extra on disk

We **do not** write `TASK.md`, `PLAN.md`, or `AGENT.json` files into worktrees. Every protocol event is a commit on the agent branch. The worktree's disk state contains only implementation deliverables.

```
.loom/projects/<project-slug>/<role>/<slug>/     ← worktree root
  <implementation files>                         ← only deliverables
```

No protocol files pollute the working tree. `git status` means what it says.

## State machine on the agent branch

Empty commits carry protocol state; file-mutating commits carry implementation. All use `Task-Status:` trailers so `git log --grep='Task-Status:'` is the audit trail.

```
ASSIGNED       (empty)   Task-Status: ASSIGNED       orchestrator writes task spec + scope in the body
  ↓
[PLAN]         (empty)   Task-Status: PLANNING       worker writes plan in the body
  ↓ plan gate — SendMessage: "plan ready"
[FEEDBACK]?    (empty)   Task-Status: NEEDS-REVISION orchestrator writes feedback in body (optional)
  ↓ (back to [PLAN v2] if needed)
[APPROVED]     (empty)   Task-Status: APPROVED       orchestrator clears the gate
  ↓ SendMessage: "implement"
<impl commits> (files)   Task-Status: IMPLEMENTING   worker's actual changes
  ↓
<final commit>           Task-Status: COMPLETED      worker signals done
```

Benefits over LOOM-as-specified:

- **Single source of truth**: task, plan, feedback all in `git log`, no file-vs-commit drift.
- **Clean worktree**: implementation deliverables only.
- **Legible history**: reviewers see the full arc in the eventual merge commit's ancestry.
- **No amend-and-force**: feedback is a new commit, not a mutation.

## Trailers replace `AGENT.json`

The assignment metadata goes into trailers on the `ASSIGNED` commit instead of a sibling JSON file:

```
task(<agent-id>): <short task description>

<full task description + acceptance criteria>

Agent-Id: <orchestrator>
Session-Id: <uuid>
Task-Status: ASSIGNED
Assigned-To: <agent-id>
Agent-Scope: <path>[,<path>...]
Agent-Dependencies: <slug>[,<slug>...]
Agent-Budget: <integer>
```

Scope validation at merge-time parses the ASSIGNED commit's `Agent-Scope:` trailer instead of reading a file. One less artifact.

## Runtime — Teams carries the messages

`TeamCreate` creates the team; `Agent(team_name=..., name=...)` spawns each worker as a persistent teammate (not a blocking subagent call). Signals that would otherwise be polled go through `SendMessage`:

- Worker finishes planning → DMs lead "plan ready, see commit X".
- Lead approves → DMs worker "implement" (or writes `[FEEDBACK]` commit + DMs "revise, see commit Y").
- Worker finishes implementation → DMs lead "done".
- Lead merges → DMs worker via `shutdown_request`.
- Teammates can DM each other: writer A pings writer B "my change at line X affects your interface".
- Reviewer teammates (Sable, Thorn) run concurrent review during implementation instead of only after.

The commit log is the audit trail. Teams messages are the phone calls.

## When to use strict LOOM vs this hybrid

| Situation | Use |
|---|---|
| Cross-repo integration with strict DAG and scope isolation | Strict LOOM (the skill's protocol) |
| Single-agent or small-team work inside bitswell-core | Hybrid (this doc) |
| Work that needs real-time writer ↔ reviewer feedback | Hybrid |
| Work where audit-first matters over velocity | Lean strict |

Default: hybrid. Escalate to strict LOOM when the blast radius grows.

## Spawning the team

```python
# Orchestrator (Shuttle-mode / bitswell lead):
TeamCreate(team_name="<slug>", description="...")
git worktree add .loom/projects/<slug>/<role>/<name> -b loom/<slug>/<role>-<name> origin/main
# or from a task branch:
git worktree add .loom/projects/<slug>/<role>/<name> -b loom/<slug>/<role>-<name> task/<slug>/<name>

# Write ASSIGNED commit with Agent-Scope, Agent-Dependencies, Agent-Budget trailers
git -C <worktree> commit --allow-empty -m "<ASSIGNED task body>"

# Spawn worker as teammate (NOT blocking Agent call)
Agent(subagent_type="moss", team_name="<slug>", name="moss", prompt="<worker brief>")

# Spawn reviewer concurrently if desired
Agent(subagent_type="sable", team_name="<slug>", name="sable", prompt="<review brief>")
```

Workers run async, report via `SendMessage`. The lead handles plan gate, merge, shutdown.

## What this hybrid drops from LOOM-as-specified

- `TASK.md` → the ASSIGNED commit body is the task.
- `PLAN.md` → a `[PLAN]` empty commit body is the plan.
- `AGENT.json` → trailers on the ASSIGNED commit.
- Blocking two-phase spawn → teammates are persistent; plan gate is a SendMessage exchange.

Everything else — scope enforcement, DAG, Task-Status trailers, Key-Finding trailers, never-force-push, topological integration — stays.

## Retrofit for existing agents

Agent definitions (`moss.md`, `ratchet.md`, etc.) already describe the workflow roughly. The hybrid doesn't require rewriting them; it just names the runtime (Teams) and clarifies that protocol events are commits, not files. Expect a one-round clarification pass on the writer agents' `.claude/agents/*.md` in a follow-up PR.
