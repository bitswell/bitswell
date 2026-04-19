# Bitswell — Claude Code Configuration

## Main Agent

**Bitswell** (`.claude/agents/bitswell.md`) is the primary agent for this project — the user-facing layer that coordinates the team. Bitswell stays at the primary worktree, always; operational work (worktree mechanics, team assembly, PR lifecycle) runs in **Shuttle-mode** — a role the top-level session adopts, not a separate process.

To invoke bitswell explicitly, use `@bitswell` or launch it as a subagent. Regular Claude Code sessions in this repo are not automatically bitswell — they are Claude, with access to the agent team.

Project identity and values are in `AGENT.md`. Agent identities are in `agents/<name>/identity.md`.

## Agent Team

| Agent | Role | When to use |
|-------|------|-------------|
| **bitswell** | Top-level agent | User interaction, strategy, dispatch. Never leaves the primary worktree |
| **shuttle** | Operational orchestrator role | Worktree mechanics, team assembly, PR lifecycle. A mode the lead adopts — not a spawnable subagent |
| **bitsweller** | Issue finder | Proactively finds optimization opportunities |
| **vesper** | Planner | Decomposes issues into implementation tasks |
| **ratchet** | Writer | Implements tasks — structural, practical |
| **moss** | Writer | Implements tasks — surgical, minimal |
| **drift** | Reviewer | Lateral thinking, intuitive review |
| **sable** | Reviewer | Skeptical, incisive review |
| **thorn** | Reviewer | Stress-testing, adversarial review |
| **glitch** | Reviewer | Chaos testing, breaks things |
| **bitswelt** | Approver | Final sign-off on implementations |

## Development Workflow

**Two layered rules:**

1. **Mechanical (pre-commit hook)** — the primary worktree (`/home/willem/bitswell/bitswell`) stays on `main` with a clean working tree. Every file-mutating change lands on `main` via a PR from a linked worktree under `.loom/projects/<project-slug>/<role>/<slug>`. The hook at `scripts/hooks/pre-commit` blocks violations (activate via `./startup.sh`, which sets `core.hooksPath=scripts/hooks`).

2. **Behavioral (who does what)** — Bitswell is the top-level agent: strategic, user-facing, decides goals. Shuttle is the **operational orchestrator role** — the mode the top-level session shifts into when handling worktree mechanics and PR work. Dispatch happens via **Agent Teams**: the lead calls `TeamCreate` and spawns writer/reviewer/approver *teammates* (not nested subagents). Writers (Moss, Ratchet) and reviewers (Drift, Sable, Thorn, Glitch) coordinate via `SendMessage` and a shared task list; Shuttle-mode drives the flow. Shuttle cannot itself be spawned as a subagent for dispatch — Claude Code subagents cannot spawn further agents. See MEMORY `feedback_subagents_cant_nest.md`.

- Agents work in git worktrees. Use standard git (branch, commit, push, PR) — no external VCS tools.
- Bitsweller files issues as commits on the `bitsweller` branch.
- Tasks live in `tasks/` (unassigned, assigned, done) — the files in these directories are protocol artifacts (see `tasks/README.md`) and must be tracked. Vesper writes them from a planner worktree; never directly at the top-level.
- Agent identities live in `agents/<name>/identity.md`. Not all agents have discovered identities yet — bitsweller and bitswelt are pending.

## Pipeline Visibility

Three git-native mechanisms track work from issue through merge:

- **`refs/notes/pipeline`** — YAML notes attached to bitsweller issue commits. Track status (`filed → planned → assigned → in-review → shipped → abandoned`), implementation PR, reviewers, and retro link. Written by bitsweller (filed), vesper (planned), bitswell (assigned), bitswelt (shipped). Fetch with `git fetch origin refs/notes/pipeline:refs/notes/pipeline`, view with `git log bitsweller --show-notes=pipeline`.
- **`retros` branch** — Orphan branch, one commit per merged PR. 5-heading template (What worked / What surprised us / What we'd do differently / Follow-ups filed / Signal for future planning). Written by bitswelt at approval time. Ceiling: 15 lines.
- **`Bitsweller-Issue: <sha>` trailer** — Added to merge commits in implementation repos (loom-tools, memctl, etc.) to close the reverse link from PR back to issue. Makes the backlink grep-exact.

These three form a closed loop: issue → note → PR → merge trailer → issue, with retro linked from the note.
