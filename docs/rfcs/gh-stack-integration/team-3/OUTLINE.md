# Team 3 Outline — gh-stack as a Projection Layer

## Angle statement

Projection-only: gh-stack runs as a post-integration review rendering over
LOOM's merge-based workspace, never as a live branching model workers touch.

## Thesis

LOOM's sharpest edges around gh-stack (rebase vs `--no-ff`, worker authority,
branch-namespace collisions) all arise from one assumption: that the stack is
where work *happens*. We reject that assumption. LOOM's merge-based
integration and its `loom/<agent>-<slug>` branches stay exactly as they are
today — the workspace is the source of truth. After integration, a separate
`stack-publish` step replays the integrated commit graph onto a disposable
parallel set of `stack/<epic>/<step>` branches and hands them to
`gh stack submit --auto --draft`. The stack is a *rendering* of the audit
trail, not the audit trail itself. This is the only angle that keeps LOOM's
invariants fully intact while still giving reviewers the ladder UI, and it is
the cheapest to ship because it never contends with `gh-stack`'s rebase model
on any branch an agent owns.

## Section headers for proposal.md

### 1. Angle statement
- Restate: stack is a projection of the merge audit trail, not a live branching scheme.
- Make the one-line claim that LOOM and gh-stack never share a branch.
- Frame the proposal as "preserve everything, add a publisher."
- Name the shippable artifact: one new `stack-publish` recipe plus zero mandatory tool changes.

### 2. What changes
- New recipe `skills/loom/recipes/stack-publish.md` driven by the orchestrator after epic integration.
- New optional `loom-tools` tool `stack-publish` (thin wrapper around `gh stack init --adopt` + `gh stack submit --auto --draft`) living at `repos/bitswell/loom-tools/src/tools/stack-publish.ts`.
- Zero changes to: worker template, `pr-create.ts`, `pr-retarget.ts`, `commit.ts`, `push.ts`, the commit schema, the `Scope` rules, or the `Dependencies` trailer semantics.
- One additive field on epic assignments: optional `Stack-Publish: true` marker the orchestrator reads to decide whether to run the recipe (not a new protocol trailer — a recipe config flag).

### 3. Branch naming and scope
- LOOM branches remain `loom/<agent>-<slug>`, untouched by gh-stack commands.
- Published stack uses a disjoint namespace `stack/<epic-slug>/<NN>-<short-label>` where `NN` is the integration order.
- `Scope:` enforcement is unchanged because no worker writes to `stack/*` branches — the orchestrator cherry-picks integrated commits onto them.
- Branch lifetime: `stack/*` branches are disposable, recreated every publish; they carry no audit value and may be force-pushed freely.
- Document that `stack/*` is the one namespace where LOOM tolerates non-linear history on remote, because it is not a source of truth.

### 4. Merge vs rebase
- LOOM's `--no-ff` integration into the workspace is preserved verbatim; the audit trail lives in merge commits on `main`.
- The published stack uses cherry-pick-onto-fresh-branches, not rebase of an existing chain, so `gh-stack`'s force-push rebase model never collides with a LOOM-owned ref.
- Reviewers reading a `stack/*` PR see the layered diff; reviewers auditing history read `main`'s merge commits. These two views are explicitly separate and the proposal names them.
- Argue that any attempt to unify the two views loses audit fidelity *or* loses stack reviewability, and this proposal refuses the false choice.
- Address the "but the commit SHAs differ" objection: each `stack/*` PR body links back to the corresponding merge commit on `main` for traceability.

### 5. Worker authority
- Workers NEVER invoke `gh stack` commands. Full stop.
- The existing invariant "only the orchestrator creates PRs" is preserved; `stack-publish` is an orchestrator-only tool (`roles: ['orchestrator']` matching `pr-create.ts`).
- No LOOM invariants are relaxed. This is the strongest authority story of any angle.
- Explicitly list the invariants preserved: workspace-write monopoly, scope enforcement at integration, no cross-agent branch writes, PR authority.

### 6. End-to-end example
- 3-agent epic: `auth-middleware` -> `api-endpoints` -> `frontend` with Dependencies trailers forming a linear chain.
- Phase A: orchestrator assigns, workers run in isolated worktrees, each commits COMPLETED on `loom/<agent>-<slug>`.
- Phase B: orchestrator integrates sequentially into workspace via `git merge --no-ff` in DAG order, producing three merge commits on `main`.
- Phase C: orchestrator runs `stack-publish` recipe — creates `stack/auth-epic/01-auth-middleware`, `stack/auth-epic/02-api-endpoints`, `stack/auth-epic/03-frontend` by cherry-picking the integrated commit ranges, then `gh stack init --adopt ...` in publish order, then `gh stack submit --auto --draft`.
- Phase D: reviewer reads the draft stack on GitHub; any review-feedback edits go through a NEW LOOM assignment (not by editing the stack), the stack is republished idempotently after re-integration.
- Show the exact sequence of tool calls: `assign`, `dispatch`, `commit` x3 agents, `integrate` x3, `stack-publish`, done.

### 7. Risks and rejected alternatives
- Risk: reviewers may be confused that comments on a `stack/*` PR don't drive code changes. Mitigation: PR body template points reviewers at the workspace repo for change requests.
- Risk: cherry-pick conflicts when merge commits touch overlapping files. Mitigation: publish operates on integrated tree states, not the merge graph, so there is no cherry-pick conflict — it is a linear replay of `main`.
- Risk: double-PR noise (both the original per-agent PR, if any, and the stack PR). Mitigation: in stack-publish mode, agents skip `pr-create` entirely and ship only via integration.
- Rejected alternative #1: **live stacking on `loom/*` branches**. Rejected because it forces workers to call `gh stack` or forces the orchestrator to force-push worker branches — either violates LOOM's trust boundary.
- Rejected alternative #2: **Stack-* trailer protocol extension**. Rejected because it bloats the schema to model a view that is already derivable from integration order, and it couples LOOM's wire format to a specific external tool.
- Rejected alternative #3: **replacing `--no-ff` merges with rebase**. Rejected because it destroys the audit trail LOOM's observability story depends on (section 8 of `protocol.md`).

## Key references

- `/home/willem/.agents/skills/gh-stack/SKILL.md` — authoritative `gh stack` command reference; note the `--adopt` flag on `init` (lines ~419-423) and `--auto --draft` on `submit` (lines ~528-535).
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` — section 6.1 (trust boundary), section 8.2 (audit trail), and section 3.3 (integrate) are the invariants this proposal preserves.
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` — section 2 (branch naming, `loom/<agent>-<slug>`) and section 3.3 (Dependencies trailer). The proposal adds NO trailers.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts` — lines 27-28 show `roles: ['orchestrator']`; the new `stack-publish` tool mirrors this exactly.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts` — full file (~42 lines) shows the minimal-wrapper pattern the new tool follows.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts` — Kahn's-algorithm topo sort (lines 142-165) produces the `integrationOrder` array the publisher uses as its `NN-` prefix order.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/index.ts` — where the new `stack-publish` tool gets registered.
