# Team 3 Outline — gh-stack as a First-Class Protocol Primitive

## Angle statement

First-class protocol primitive: extend the LOOM commit trailer vocabulary with new
`Stack-*` trailers so that stack membership, position, and base are wire-format
facts the dispatcher, validators, and integrator all reason about — rather than a
convention layered on top of the audit trail after the fact.

## Thesis

The other four angles all try to keep LOOM's wire format frozen and bolt `gh-stack`
onto it sideways — either as a publisher, a recipe, or a convention. Every one of
those choices pushes the cost of representing a stack into tooling that has to
re-derive it on every invocation, and every one of them leaks on the first hard
case: mid-stack rework, DAG fan-out that is not a pure chain, or a worker that
needs to know what layer it is in to write its own commit range correctly.

Pay the protocol debt now. A stack is a first-class object in the workers' world,
not a review-time rendering. The natural abstraction layer is the one LOOM already
uses for every other cross-agent invariant: commit trailers. Adding `Stack-Id`,
`Stack-Position`, `Stack-Base`, and `Stack-Epic` to the vocabulary lets the
existing validators (`trailer-validate`, `lifecycle-check`), the dispatcher, the
integrator, and `dag-check` enforce stack invariants *the same way they enforce
every other invariant today* — by reading git. Once stacks are trailer-native,
`gh-stack` itself becomes a thin projection that reads trailers and renders a
linear chain, rather than a second source of truth in constant tension with the
first.

The cost is real: four new trailers, new rules in `schemas.md`, new checks in
`trailer-validate.ts` and `lifecycle-check.ts`, new dispatch-time topology
resolution, a new `Stack-Base-SHA` notion for rebase reconciliation, and an
`mcagent-spec.md` conformance update. The payoff is that every sharp edge the RFP
lists (§1–§5) either disappears or collapses into a single rule in one validator.
The alternative — keeping the schema frozen — pays the same debt eventually, but
pays it in tooling that drifts from the protocol instead of in the protocol
itself. That is the worse debt.

## Section headers for proposal.md

### 1. Angle statement

- One sentence: stack topology is a wire-format fact, encoded as `Stack-*`
  trailers on LOOM commits, enforced by the same validators that enforce
  lifecycle today.
- Frame the proposal as "the MAXIMUM protocol-change angle" explicitly, so the
  reviewer can compare it against the minimum-change angles the other teams
  take.
- Name the non-negotiable claim: if an epic is stack-mode, every ASSIGNED commit
  for its workers carries a `Stack-Id` and a `Stack-Position`, and the
  validators MUST reject an assignment that omits them.
- Commit to shippability: the schema changes are additive (optional on
  non-stack epics), so non-stack LOOM work is unaffected and the upgrade is
  rolling.

### 2. What changes

- **New trailers in `schemas.md` §3 (trailer vocabulary)** — four additions:
  - `Stack-Id: <uuid>` — stable identifier for the stack this commit belongs
    to; identical across every commit on every branch in the stack. Required on
    ASSIGNED commits when the assignment is stack-mode. Required on every
    subsequent commit on a stack-mode branch (auto-injected by `commit.ts` once
    observed on the branch, mirroring how `Session-Id` already auto-propagates).
  - `Stack-Position: <integer>` — 1-indexed position of this branch within its
    stack, counted from the bottom (closest to trunk). Required on stack-mode
    ASSIGNED commits. Must be unique within a `Stack-Id`.
  - `Stack-Base: <agent>/<slug> | trunk` — the immediate downstack neighbor (or
    `trunk` if this is the bottom of the stack). Resolves to a LOOM branch via
    the existing `<agent>/<slug>` -> `loom/<agent>-<slug>` mapping in
    `schemas.md` §2. Required on stack-mode ASSIGNED commits. For the bottom
    branch, value MUST be `trunk`.
  - `Stack-Epic: <slug>` — human-readable name of the epic the stack belongs
    to; used by the dispatcher for logging and by the integrator for
    commit-message generation. Required on stack-mode ASSIGNED commits. MUST
    be stable across all members of one `Stack-Id`.
- **Optional trailer: `Stack-Base-SHA: <sha>`** on COMPLETED commits — records
  the integrator-observed SHA of the downstack neighbor at the moment this
  branch was integrated. Used by rebase reconciliation (§4) to detect when a
  downstack edit forces an upstack rebase and to generate the force-push.
- **`schemas.md` validation rules** — new rules in §7:
  - `stack-id-format` — `Stack-Id` MUST be UUID v4.
  - `stack-position-positive` — integer >= 1.
  - `stack-position-unique` — within a `Stack-Id`, each position appears on at
    most one branch (enforced at dispatch time and integration time).
  - `stack-base-resolvable` — `Stack-Base` value either equals `trunk` or
    resolves to an existing `loom/*` branch with the same `Stack-Id`.
  - `stack-base-position` — `Stack-Base`'s `Stack-Position` equals this
    branch's position minus 1 (or `Stack-Base` is `trunk` and position == 1).
  - `stack-trailers-inheritance` — once a branch has observed `Stack-Id` on
    its ASSIGNED commit, every subsequent commit MUST carry the same
    `Stack-Id` and `Stack-Position` (mirrors how `Session-Id` works but for
    stack identity).
  - `stack-dependencies-consistency` — if an ASSIGNED commit has a
    `Stack-Base` pointing at `<agent>/<slug>`, then `Dependencies` MUST
    include that exact ref. Eliminates the "two ways to say the same thing"
    footgun by making the stack ordering a stronger version of the dependency
    ordering.
- **`trailer-validate.ts` changes** — new rule functions following the pattern
  of `scope-expand-format` (lines 279-305):
  - Enum/format checks for the four trailers.
  - ASSIGNED-state block (lines 213-247) gains stack-mode detection: if
    `Stack-Id` is present, require `Stack-Position`, `Stack-Base`, `Stack-Epic`.
  - New `TASK_STATUS_ENUM` is unchanged; PLANNING stays the gate for the new
    `stack-planning-*` rules that make the planner commit pin stack layout.
- **`lifecycle-check.ts` changes** — branch-level invariants:
  - New rule `lifecycle-stack-inheritance`: walk `base..branch`, assert every
    commit after ASSIGNED carries the same `Stack-Id` the ASSIGNED commit set.
  - New rule `lifecycle-stack-position-stable`: `Stack-Position` MUST NOT
    change within a branch's lifecycle.
  - Extension of `TERMINAL_STATES` semantics (lines 36-39): post-terminal
    orchestrator commits on a stack branch MUST preserve `Stack-Id` (otherwise
    the projection layer desyncs).
- **`dispatch.ts` changes** — stack-aware dispatch gating:
  - Before spawning a worker for a stack-mode branch, read the ASSIGNED
    commit's `Stack-Base`. If base is not `trunk`, verify the base branch has
    reached `COMPLETED` (mirrors existing dependency check but via
    `Stack-Base` instead of/in addition to `Dependencies`).
  - Populate a `STACK_*` env var block the worker template can read, so
    workers know their position without re-parsing trailers: `STACK_ID`,
    `STACK_POSITION`, `STACK_BASE_BRANCH`, `STACK_EPIC`.
  - New dispatch-time rule: refuse to spawn if `stack-position-unique` fails
    across the set of unintegrated ASSIGNED commits for this `Stack-Id`.
- **`dag-check.ts` changes** — stack-aware topological order:
  - New input field: per-agent `stackPosition?: number` and `stackId?: string`.
  - When agents share a `Stack-Id`, their relative integration order MUST
    match ascending `Stack-Position`. Violations emit a new
    `dag-stack-order-violation` rule.
  - Existing cycle detection is unchanged; stacks are just a stronger
    ordering constraint on top of the DAG.
- **`commit.ts` changes** — trailer auto-injection:
  - Lines 66-72 already auto-inject `Agent-Id`, `Session-Id`, `Heartbeat`. Add
    a read-from-branch step: if the branch's ASSIGNED commit has `Stack-Id`,
    auto-inject `Stack-Id` and `Stack-Position` on every subsequent commit.
    Eliminates the class of bug where a worker forgets to propagate the
    trailer.
- **`mcagent-spec.md` conformance rule** — one new MUST: a conforming agent
  runtime MUST read the ASSIGNED commit's stack trailers, set `STACK_*` env
  vars for the worker, and MUST NOT silently strip stack trailers from
  derivative commits.
- **`protocol.md` §7 (coordination)** — one new paragraph: if an assignment
  carries `Stack-Base: <agent>/<slug>`, that ref MUST also appear in
  `Dependencies`. The stack is the contract; the dependency graph is a
  consequence.
- **Zero changes** to: branch naming (`loom/<agent>-<slug>` stays as-is — the
  stack is a layer above branch names, not a rename), the state machine in
  `lifecycle-check.ts` (stacks operate orthogonally to `ASSIGNED -> PLANNING
  -> IMPLEMENTING -> COMPLETED`), scope enforcement semantics (each worker
  still has its own `Scope`), or the `Session-Id` model.

### 3. Branch naming and scope

- Branch names stay `loom/<agent>-<slug>`. Stack membership is a trailer fact,
  not a branch-name fact. This is the key insight: LOOM's branch namespace is
  already per-agent and per-assignment; overloading it with stack position
  (`stack/<epic>/<nn>-*`) would force a rename on every rework.
- A worker's `Scope:` trailer is unchanged — stacks do not relax scope. Each
  layer's worker writes only its own files. The integrator's existing scope
  check at integration time (`protocol.md` §3.3) is untouched.
- Cross-layer visibility: a worker MAY read the downstack branch's tree during
  its own IMPLEMENTING phase (the git-worktrees model already permits this via
  `git show`). The `Stack-Base` trailer is the pointer that makes "which
  branch is below me" a protocol fact instead of a filesystem guess.
- A new failure mode appears at dispatch: if two ASSIGNED branches claim the
  same `Stack-Id` + `Stack-Position`, `dispatch.ts` refuses both and emits a
  `stack-position-unique` violation. Caller fixes by resetting one of the
  ASSIGNED commits. This is a *better* error story than two branches silently
  colliding in a projection layer.
- `Scope-Expand` (existing trailer) continues to work unchanged within a
  layer. A layer's worker CANNOT use `Scope-Expand` to reach into a downstack
  layer's files — that would require a new assignment at the correct layer,
  exactly as the `gh-stack` skill recommends in its "navigate down the stack"
  rule.

### 4. Merge vs rebase

- The `Stack-Base-SHA` trailer is the pivot. On COMPLETED, the integrator
  records the SHA of the downstack branch *at the moment this branch was
  integrated*. This is the "merge base" in the audit trail.
- On integration, the orchestrator uses `git merge --no-ff` as today — the
  audit trail is preserved verbatim. The `Stack-Base-SHA` gives us the
  equivalent of `gh stack`'s rebase pointer without rewriting any history.
- When a downstack layer is amended (e.g., rework during review), the
  integrator re-integrates it as a new merge commit on the workspace. The
  upstack layer's `Stack-Base-SHA` now refers to a stale commit. This is
  *detectable*: a new `lifecycle-check` rule `stack-base-stale` walks the
  integration log, compares each layer's `Stack-Base-SHA` to the current HEAD
  of its `Stack-Base` branch, and flags stale bases.
- Resolution: the orchestrator spawns a rebase-replay worker at the affected
  layer. This is a new assignment (new `Session-Id`), not a rewrite of the
  old one. The new ASSIGNED commit inherits the same `Stack-Id` and
  `Stack-Position` but gets a fresh `Stack-Base-SHA` once integrated. The old
  branch stays in history as a record of the pre-rebase state.
- `gh-stack` projection: after integration, a thin read-only projector walks
  `Stack-Id` groups, sorts by `Stack-Position`, picks the latest COMPLETED
  branch per position, and feeds `gh stack init --adopt` with the resulting
  chain. The projector never force-pushes `loom/*` branches — it publishes to
  a disjoint `stack/<epic-slug>/<NN>` namespace exactly as Team 1's angle
  proposes. The difference is that the topology it publishes is derived from
  trailers, not from filesystem conventions.
- The RFP objection "gh-stack uses rebase + force-push, LOOM uses `--no-ff`"
  dissolves: `gh-stack` only ever sees the projection namespace, and that
  namespace is *explicitly* disposable. `loom/*` branches never see a rebase
  or a force-push, so the LOOM audit trail invariant is preserved.

### 5. Worker authority

- Workers NEVER invoke `gh stack` commands directly. The invariant
  "only the orchestrator creates PRs" extends to "only the orchestrator
  publishes stacks". Workers only read their own `STACK_*` env vars.
- A worker's authority is strictly increased in one way: a stack-mode worker
  MAY read the tree of its `Stack-Base` branch (via `git show`) to understand
  the baseline it is building on. This is already legal today via
  cross-worktree git reads; the proposal just makes it a documented part of
  the worker contract instead of an accident.
- A worker's authority is strictly constrained in one way: it MUST NOT emit
  commits that change `Stack-Id` or `Stack-Position`. `trailer-validate`
  rejects such commits via `stack-trailers-inheritance`. This is the
  worker-authority invariant the RFP asks about — stack identity is
  orchestrator-owned, even on the worker's own branch.
- Explicit invariants preserved: workspace-write monopoly (orchestrator only),
  scope enforcement at integration (unchanged), no cross-agent branch writes
  (unchanged), PR authority (unchanged), `Session-Id` per-invocation uniqueness
  (unchanged).
- Explicit invariants *added*: stack-identity monopoly (orchestrator writes
  the initial `Stack-Id`/`Stack-Position`, workers propagate them
  verbatim), stack-base stability (a worker's `Stack-Base` does not change
  mid-lifecycle).
- The new dispatch-time gate (`Stack-Base` must be COMPLETED) is a
  strictly stronger version of the existing `Dependencies` gate. It is not a
  relaxation.

### 6. End-to-end example

Trace a 3-agent epic `auth-stack` with the stack `auth-middleware` →
`api-endpoints` → `frontend`. The proposal MUST show, for each phase, the exact
trailers on the exact commits.

- **Phase 0 — planning.** Orchestrator decides the epic is stack-mode, mints a
  `Stack-Id` (e.g., `8f3b2e1a-...`), assigns the three workers. Each ASSIGNED
  commit on `loom/ratchet-auth-middleware`, `loom/moss-api-endpoints`, and
  `loom/ratchet-frontend` carries:
  - `Stack-Id: 8f3b2e1a-...`
  - `Stack-Epic: auth-stack`
  - `Stack-Position: 1` / `2` / `3`
  - `Stack-Base: trunk` / `ratchet/auth-middleware` / `moss/api-endpoints`
  - `Dependencies: none` / `ratchet/auth-middleware` / `moss/api-endpoints`
    (consistent with `Stack-Base` per the new `stack-dependencies-consistency`
    rule)
- **Phase 1 — dispatch.** `dispatch.ts` scans ASSIGNED commits. Position-1 is
  ready (base is `trunk`), positions 2 and 3 are gated on their respective
  `Stack-Base` branches reaching COMPLETED. Only position 1 spawns. The
  worker's env has `STACK_ID=8f3b2e1a-...`, `STACK_POSITION=1`,
  `STACK_BASE_BRANCH=main`, `STACK_EPIC=auth-stack`.
- **Phase 2 — layer 1 work.** Ratchet on `loom/ratchet-auth-middleware`
  commits `PLANNING` then `IMPLEMENTING` then `COMPLETED`. Every commit
  carries `Stack-Id: 8f3b2e1a-...` and `Stack-Position: 1`, auto-injected by
  `commit.ts` from the branch state. The COMPLETED commit gets
  `Stack-Base-SHA: <sha-of-trunk-at-integration>` added by the integrator.
- **Phase 3 — layer 2 dispatch.** Position 1 is COMPLETED + integrated; the
  dispatcher's `Stack-Base`-ready check for position 2 now passes. Moss spawns
  on `loom/moss-api-endpoints` with `STACK_BASE_BRANCH=loom/ratchet-auth-middleware`.
  It reads that branch's tree to understand the types/handlers it depends on,
  then writes its own files inside its `Scope`.
- **Phase 4 — layer 3.** Same pattern; ratchet spawns again on
  `loom/ratchet-frontend` with `STACK_BASE_BRANCH=loom/moss-api-endpoints`.
- **Phase 5 — rework.** Reviewer finds a bug in layer 1. Orchestrator creates
  a new ASSIGNED commit on a new branch `loom/moss-auth-middleware-fix` with
  the SAME `Stack-Id` and `Stack-Position: 1` — but `dispatch.ts` detects the
  `stack-position-unique` violation *across unintegrated commits* and refuses.
  The correct pattern is to mark the old branch SUPERSEDED via an orchestrator
  chore commit that sets `Stack-Id` to the empty/retired value and
  re-dispatch. (Proposal discusses this case as a known sharp edge and
  proposes a `Stack-Supersedes: <old-branch-sha>` trailer as a follow-up.)
- **Phase 6 — projection.** Orchestrator invokes `stack-project` (new recipe)
  which reads all COMPLETED branches with `Stack-Id: 8f3b2e1a-...`, groups by
  position, picks the latest per position, cherry-picks onto
  `stack/auth-stack/01-auth-middleware`, `/02-api-endpoints`, `/03-frontend`,
  and calls `gh stack init --adopt ... && gh stack submit --auto --draft`.
- Show the complete trailer blocks for three representative commits
  (ASSIGNED on position 2, IMPLEMENTING on position 2, COMPLETED on position
  2) in the end-to-end example, so a reviewer can run `trailer-validate`
  against them mentally.

### 7. Risks and rejected alternatives

- **Risk: schema bloat.** Four new trailers is nearly a 20% increase in the
  universal-plus-state vocabulary (3.1–3.5 currently define ~16 trailers).
  Mitigation: all four are conditional on stack-mode, and non-stack LOOM work
  is unaffected. Rolling upgrade path: old validators ignore unknown trailers.
- **Risk: validator drift.** `trailer-validate.ts`, `lifecycle-check.ts`,
  `dispatch.ts`, and `dag-check.ts` all grow new code paths; a bug in one
  could let an invalid stack slip through. Mitigation: every new rule gets a
  Rust integration test per the project's testing preference, plus a fuzzer
  over random stack shapes.
- **Risk: rebase reconciliation is not fully automatic.** The `Stack-Base-SHA`
  staleness detector flags the problem, but resolving it still requires a new
  orchestrator-spawned worker. Mitigation: this is exactly what LOOM already
  does for every other kind of rework. We are not inventing a new paradigm.
- **Risk: stack-position-unique enforcement across unintegrated ASSIGNED
  commits introduces a global check.** Today, dispatch is mostly per-branch.
  Mitigation: the check is O(N) in the number of unintegrated ASSIGNED
  commits for one `Stack-Id`, which is bounded by the depth of a stack
  (realistically <10). No cross-epic global state needed.
- **Risk: interaction with the existing `Scope-Expand` trailer.** A worker
  that scope-expands into a file owned by a downstack layer could silently
  break the stack. Mitigation: add a new `trailer-validate` rule
  `scope-expand-cross-stack` that rejects `Scope-Expand` paths which overlap
  any downstack layer's integrated changes for the same `Stack-Id`. This is
  a strict-mode check; non-strict mode warns.
- **Rejected alternative #1: encode stack position in the branch name**
  (e.g., `loom/<agent>-<slug>-s<nn>`). Rejected because it makes branch names
  carry topology, forcing a rename on every position change, and it breaks
  the clean 1:1 mapping between `loom/<agent>-<slug>` branches and
  assignment slugs in `schemas.md` §2.
- **Rejected alternative #2: a single `Stack: <epic>/<position>/<base>`
  trailer** packing all four fields. Rejected because it's harder to
  validate (one regex for four invariants), harder to query via
  `git log --format='%(trailers:key=...,valueonly)'`, and because the existing
  trailer vocabulary favors narrow single-purpose trailers.
- **Rejected alternative #3: a new `.loom/stacks/<uuid>.yaml` sidecar file**
  to store stack topology out-of-band. Rejected because it creates a second
  source of truth alongside git, requires new synchronization primitives, and
  defeats the "git is the database" invariant the LOOM protocol is built on.
- **Rejected alternative #4: piggyback on `Dependencies` alone.**
  `Dependencies` already expresses ordering, so why add `Stack-Base`?
  Rejected because `Dependencies` expresses a DAG with possible fan-out;
  `Stack-Base` asserts a *linear chain* — a strictly stronger constraint.
  `gh-stack` itself enforces strict linearity (SKILL limitation §1), so the
  protocol must express it natively. The `stack-dependencies-consistency`
  rule keeps the two in sync.

## Key references

- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md`
  — **primary extension target**. Section 3 (trailer vocabulary) gains four
  new entries; section 4 (required trailers per state) gains a stack-mode
  column; section 7 (validation rules) gains rules 15–21 for the new
  invariants.
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md`
  — section 3.3 (integrate) is the hook for `Stack-Base-SHA` recording;
  section 7 (coordination) gains the new stack-consistency paragraph;
  section 6.1 (trust boundary) gains the stack-identity-monopoly rule.
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/mcagent-spec.md`
  — gains one new MUST: conforming runtimes propagate `Stack-*` trailers and
  set the `STACK_*` env var block when spawning workers.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/trailer-validate.ts`
  — new rule functions added in the pattern of `scope-expand-format`
  (lines 279-305). Specifically: after the ASSIGNED block (lines 213-247),
  add a stack-mode detection block that requires the four trailers when
  `Stack-Id` is present. Extend `TASK_STATUS_ENUM` is NOT needed.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/lifecycle-check.ts`
  — new branch-level rules: `lifecycle-stack-inheritance` (walk commits,
  assert every post-ASSIGNED commit carries the same `Stack-Id`),
  `lifecycle-stack-position-stable`, `stack-base-stale`. Add after the
  existing post-walk invariant block (lines 337-363).
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dispatch.ts`
  — add the `Stack-Base`-ready check before the `worktree add` call (around
  line 55). Populate `STACK_*` env vars in the dispatch output so the runtime
  adapter forwards them to the worker.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/commit.ts`
  — extend the trailer auto-injection block (lines 66-72) to read the
  ASSIGNED commit's `Stack-Id` and auto-propagate it, in the same way
  `Session-Id` is already propagated.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts`
  — extend the `AgentEntry` schema (lines 5-10) with optional `stackId` and
  `stackPosition` fields. After Kahn's algorithm (lines 142-165), verify
  that agents sharing a `Stack-Id` appear in `integrationOrder` in ascending
  `Stack-Position`. Emit `dag-stack-order-violation` if not.
- `/home/willem/.agents/skills/gh-stack/SKILL.md` — reference for the
  projection-layer command sequence: `gh stack init --adopt`,
  `gh stack submit --auto --draft`, and the limitation §1 (stacks are
  strictly linear) that justifies the `Stack-Base` linear-chain invariant.
