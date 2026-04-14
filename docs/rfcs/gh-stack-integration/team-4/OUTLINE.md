# Team 4 — OUTLINE

## Angle statement

**Post-hoc projection: `gh-stack` adopts already-completed LOOM branches as a read-only reviewability lens; LOOM's DAG and `--no-ff` integration remain the sole source of truth.**

(27 words.)

## Thesis

Every other plausible integration (new trailers, MCP wrappers, stack-mode
integration, convention-only recipes) has to reconcile two models that pull in
opposite directions: LOOM treats merges as audit facts and forbids worker PR
authority, while `gh-stack` rebases, force-pushes, and assumes the branch
author drives submission. This proposal refuses the reconciliation. LOOM runs
to completion exactly as it does today — workers commit, orchestrator
`--no-ff` merges onto `main` — and **only after** an epic's DAG has finished
integrating, the orchestrator runs `gh stack init --adopt` against the same
branches it just merged, purely to publish a stacked PR chain for human
reviewers. The stack is a projection of history, not a mode of work. This
angle is the right bet because it is the only one that leaves both systems
unmodified in their hot paths: zero protocol changes, zero worker changes,
zero merge-semantics changes, and the merge-vs-rebase incompatibility simply
never arises because rebases happen on read-only branches after the merge is
already a fact on `main`.

## Section headers for `proposal.md`

The writer must fill in all seven RFP-required sections in the order below.
Each header lists 2–5 bullet claims the writer must argue.

### 1. Angle statement

- Restate the one-sentence angle verbatim: post-hoc projection; `gh-stack` is
  a read-only reviewability lens over a completed LOOM epic.
- Name the collision-detection key clearly: *projection stacking*, *post-hoc
  adoption*, or *downstream-only stacking* — all three refer to the same
  angle.
- Contrast explicitly with angles the writer should assume other teams will
  pick (new trailers, MCP wrappers, stack-mode integration, convention
  recipes) and say why none of them share this bet.

### 2. What changes

- **`loom-tools`**: add exactly one new tool, `stack-publish`, with
  `roles: ['orchestrator']`, whose entire job is to run
  `gh stack init --adopt` over a list of already-merged LOOM branches and
  `gh stack submit --auto --draft` to open the PR chain. No new tool wraps
  any worker-facing stack operation.
- **Plugin/skill surface**: add a `publish-stack` recipe to the `loom` skill
  that the orchestrator invokes after epic integration. No worker-template
  change. No schema change.
- **Schemas**: **zero changes**. No new trailers. No new `Task-Status`
  values. The writer should quote `schemas.md §3` to prove this.
- **File paths to touch** (writer lists them concretely): new
  `repos/bitswell/loom-tools/src/tools/stack-publish.ts`, registration in
  `repos/bitswell/loom-tools/src/tools/index.ts`, and a new recipe under
  `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/`
  (writer confirms exact recipe-dir path).
- **Non-goal to state explicitly**: neither `pr-create.ts` nor
  `pr-retarget.ts` is modified; they continue to do single-PR work and are
  the *only* PR surface during integration.

### 3. Branch naming and scope

- LOOM branches keep their canonical `loom/<agent>-<slug>` form through
  integration. `gh-stack`'s prefix model is **never applied during work** —
  only at adoption time, and even then only as a projection.
- `gh stack init --adopt loom/<a>-<s1> loom/<b>-<s2> ...` is run without
  `-p`, so adopted branches retain their names; `gh-stack` sees them as a
  bare chain of existing refs, which the SKILL explicitly supports.
- `Scope:` enforcement is **unchanged** because the stack is built only from
  branches whose scopes LOOM has already validated at integration time; the
  adoption step never reads or rewrites file paths.
- The writer must address the edge case where a LOOM epic includes agents
  whose branches have *already been force-deleted* post-merge and show how
  `stack-publish` detects this and refuses the projection.

### 4. Merge vs rebase

- The merge-vs-rebase tension **does not exist** in this angle: LOOM's
  `--no-ff` merge onto `main` happens first, the audit trail is already a
  fact, and only then does `gh-stack` rebase.
- Rebases performed by `gh-stack` operate on the *adopted* refs, which at
  that point are historical branch tips behind `main`. The writer must show
  that force-pushes from `gh stack submit` touch only the adopted branches,
  not `main`, and therefore cannot corrupt the audit log.
- Risk to name: if a human later runs `gh stack rebase` on a published stack
  after new commits have landed on `main`, the adopted branches get
  retargeted in a way LOOM did not authorize. Mitigation: `stack-publish`
  marks the stack with a label `loom-projection` and a repo-level guard hook
  rejects further `gh stack` operations on labelled stacks.
- Compare-and-contrast paragraph: show why a rebase-first angle must either
  rewrite history on `main` or abandon `--no-ff`, and quote LOOM's audit
  invariant to justify refusing that trade.

### 5. Worker authority

- Workers **never** invoke any `gh stack` command. The worker template,
  `mcagent-spec.md`, and the `roles: ['orchestrator']` constraint on every
  PR-related tool in `loom-tools` all remain in force unchanged.
- The new `stack-publish` tool is gated with `roles: ['orchestrator']`
  exactly as `pr-create` and `pr-retarget` already are — the writer should
  cite the `roles` field in `pr-create.ts` line 26 and `pr-retarget.ts`
  line 24 as precedent.
- No LOOM invariant is relaxed. The writer must state this explicitly as a
  *feature* of this angle, contrasting with any proposal that lets workers
  run `gh stack` commands.
- Spell out what happens if a worker accidentally runs `gh stack` anyway:
  the command fails on the worktree's isolated ref layout, and even if it
  succeeded it would be invisible to the orchestrator, which re-adopts
  branches from scratch at publish time.

### 6. End-to-end example

Trace the canonical `auth → api → frontend` three-agent epic. The writer
must cover every numbered step in order:

- Epic decomposition: orchestrator creates three ASSIGNED commits on
  `loom/ratchet-auth-middleware`, `loom/moss-api-endpoints`,
  `loom/ratchet-frontend`, with `Dependencies:` wired `none`,
  `ratchet/auth-middleware`, `moss/api-endpoints`.
- Parallel-or-serial work phase: show `dag-check` producing the integration
  order, workers committing IMPLEMENTING → COMPLETED with Heartbeats.
- Integration phase: orchestrator does `--no-ff` merge of
  `loom/ratchet-auth-middleware` into `main`, then `loom/moss-api-endpoints`,
  then `loom/ratchet-frontend`. Standard LOOM. Show the three merge commits.
- Publish phase: orchestrator invokes `stack-publish` with the integration
  order. Under the hood: `gh stack init --adopt` over the three branches,
  `gh stack submit --auto --draft` to create three draft PRs whose bases
  chain `main` → auth → api → frontend.
- Post-publish: human reviewer reads the three draft PRs top-down as a
  coherent ladder; merging any PR is a no-op on `main` (already merged) and
  the orchestrator closes the draft stack with a follow-up tool call.

### 7. Risks and rejected alternatives

- **Risk 1 — reviewer confusion**: stacked PRs that appear after their
  commits already landed may confuse reviewers used to pre-merge review.
  Mitigation: draft-only, labelled `loom-projection`, PR body explicitly
  names them "historical review ladders."
- **Risk 2 — branch-lifetime coupling**: the projection requires LOOM
  branches to still exist at publish time; orchestrator's existing
  post-merge branch cleanup must be deferred until after `stack-publish`.
  Writer must name the exact cleanup step in LOOM that has to move.
- **Risk 3 — stack drift if `main` advances**: if new commits land on `main`
  between the last `--no-ff` merge and `stack-publish`, adoption still
  works but the adopted branches are slightly behind. Writer must show the
  one-line fix (adopt from the merge commits, not the branch tips).
- **Rejected alternative A — live stacking during work**: require workers
  to run `gh stack add` as they go. Rejected because it forces worker PR
  authority and contradicts `roles: ['orchestrator']` precedent across
  `loom-tools`.
- **Rejected alternative B — protocol-level `Stack-Position` trailer**: add
  a new trailer that declares stack order at assignment time. Rejected
  because `Dependencies:` already encodes the chain and `dag-check.ts`
  already computes the topological order; adding a parallel mechanism
  violates the RFP's reusability criterion.
- **Rejected alternative C — replace merge with rebase entirely**: abandon
  `--no-ff` merges and switch LOOM to rebase-only integration so stacks
  are native. Rejected because it destroys the audit trail LOOM's protocol
  document names as a core invariant.

## Key references

The writer must read these files before writing `proposal.md`. Paths are
absolute unless noted.

- **Epic RFP (authoritative requirements)**: `gh issue view 74 --repo bitswell/bitswell` — the seven required sections and five sharp edges are quoted from here.
- **`gh-stack` SKILL**: `/home/willem/.agents/skills/gh-stack/SKILL.md` — read the "Agent rules" section for non-interactive flag requirements, the "Workflows → End-to-end" section for `init --adopt` semantics, and the "Quick reference" table for `gh stack init --adopt branch-a branch-b` syntax used by `stack-publish`.
- **LOOM protocol**: `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` — §2 state machine (IMPLEMENTING → COMPLETED), §3.3 `integrate()` (the `--no-ff` merge step this proposal leaves untouched), §4.2 dispatch.
- **LOOM schemas**: `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` — §3 trailer vocabulary (cite to prove zero additions), §3.3 `Dependencies` trailer (the DAG this proposal reuses), §2 branch-naming pattern (`loom/<agent>-<slug>`).
- **`pr-create.ts`**: `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts` — line ~26, `roles: ['orchestrator']` is the precedent `stack-publish` must follow; the whole file is the template for the new tool.
- **`pr-retarget.ts`**: `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts` — line ~24, same `roles: ['orchestrator']` guard; shows how the existing tool already supports arbitrary base branches and therefore needs no modification.
- **`dag-check.ts`**: `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts` — the Kahn's-algorithm topological sort whose `integrationOrder` output this proposal reuses as the stack order at publish time.
- **Tool registration**: `repos/bitswell/loom-tools/src/tools/index.ts` (writer confirms exact path) — where `stack-publish` must be registered alongside the existing PR tools.
