# Team 5 — OUTLINE

## Angle statement

Dual-topology review projection: LOOM's `--no-ff` merge history stays the
canonical integration truth, and `gh-stack` produces a derived, ephemeral
reviewer-facing stack on a parallel `review/<epic>/*` namespace.

## Thesis

The sharp edge everyone will trip on is merge-vs-rebase: LOOM needs `--no-ff`
audit trails, `gh-stack` needs rebase + force-push. Every other plausible angle
(zero-code conventions, new trailers, stack-mode-replaces-merge) is forced to
pick one side and compromise the other. We refuse the dichotomy by treating
stacking as a *projection* of the DAG — a throwaway review artifact the
orchestrator synthesizes from already-integrated work, never an integration
mechanism. The canonical branches (`loom/<agent>-<slug>`) stay merge-based and
untouched; the `review/<epic>/*` branches are rebased, force-pushed, and
disposable. Workers never run `gh stack`, scope rules stay pointwise, and the
audit trail is preserved byte-for-byte.

## Section headers for proposal.md

### 1. Angle statement
- Restate the dual-topology claim in one sentence at the top of the doc.
- Name the two topologies explicitly: **canonical** (loom/<agent>-<slug>,
  merge-based) and **review** (review/<epic>/<n>-<slug>, rebase-based,
  ephemeral).
- Assert that only the orchestrator ever writes to the review topology and that
  no LOOM invariant is weakened.
- State the one-line tradeoff: reviewers get the stacked-PR UX, LOOM pays the
  cost of materialising a second branch namespace per epic.

### 2. What changes
- **`loom-tools`**: add ONE new orchestrator-only tool, `stack-project`, that
  reads the epic DAG from `dag-check` and writes the `review/<epic>/*`
  namespace via `gh stack init --adopt` on temporary rebased copies. Cite
  `repos/bitswell/loom-tools/src/tools/dag-check.ts` (Kahn's topo sort, lines
  ~140-165) as the input source.
- **No changes** to `pr-create.ts` (lines 32-45 already accept arbitrary
  `base`) — we reuse it when the orchestrator later retargets review PRs.
- **`loom` plugin/skill**: add a new orchestrator recipe,
  `stack-publish-review`, that runs after an epic's agents are all COMPLETED
  and *after* the canonical merge into the workspace.
- **Worker template**: no changes. Workers stay unaware of stacking.
- **Schemas**: add exactly one optional trailer, `Review-Stack`, on the
  orchestrator's stack-publish commit so the review topology is discoverable
  from git log. No worker trailers change.

### 3. Branch naming and scope
- Keep `loom/<agent>-<slug>` untouched; it remains the integration namespace.
- Introduce `review/<epic-slug>/<nn>-<agent>-<slug>` for the projected stack
  (the `<nn>` prefix preserves topological order so `gh stack init --adopt`
  sees branches in dependency order).
- `Scope` trailers are unchanged: each canonical worker branch still enforces
  pointwise scope; review branches have no `Scope` because they are derived,
  not authored.
- Name collisions: the `review/*` prefix is disjoint from `loom/*`, so no
  branch-name ownership fight with `gh-stack`'s own `-p` prefix model.
- Cleanup: review branches are deleted automatically once the epic's canonical
  merge hits `main`.

### 4. Merge vs rebase
- **Canonical path is unchanged**: `--no-ff` merges into the workspace, full
  audit trail preserved, `git log --first-parent main` still tells the LOOM
  story.
- **Review path is rebase-only**: `gh stack` force-pushes `review/*` branches
  freely; the audit trail there is explicitly disposable.
- The projection is idempotent: running `stack-project` twice on the same DAG
  produces equivalent review branches (modulo commit SHAs), which is fine
  because nothing depends on review SHAs being stable.
- Justify the duplication cost: two namespaces is cheaper than corrupting
  either audit mode.
- Counter to "one-source-of-truth" objection: the canonical topology IS the
  truth; the review topology is a view, like a SQL materialised view.

### 5. Worker authority
- Workers NEVER invoke `gh stack`. Not once. No invariant is relaxed.
- The orchestrator is the sole caller of `gh stack init/add/submit/sync` and
  does so only from a dedicated `review/*` working tree.
- Justify with the LOOM trust boundary table in protocol.md §6.1: "Workspace
  write — only the orchestrator writes." The review topology is a second
  write-restricted surface under the same rule.
- Explicitly reject the "let workers stack as they go" approach — it would
  require relaxing the workspace-write invariant AND the no-cross-worktree
  invariant at the same time.
- Note that `gh stack submit --auto --draft` is safe for automation because
  the orchestrator controls titles via commit subjects on the projected
  branches.

### 6. End-to-end example
- Concrete epic: "add auth middleware -> API endpoints -> frontend" (the
  three-agent example from the RFP).
- Phase A (decomposition, unchanged): orchestrator opens epic #N, assigns
  `ratchet/auth-mw`, `moss/api-endpoints` (deps: ratchet/auth-mw), and
  `drift/frontend` (deps: moss/api-endpoints). `dag-check` returns
  topological order.
- Phase B (canonical work, unchanged): three workers commit on their
  `loom/<agent>-<slug>` branches, each reach COMPLETED, orchestrator
  integrates via `--no-ff` merges into the workspace in dependency order.
- Phase C (projection, new): orchestrator runs `stack-project --epic N`.
  For each node in topo order, it cherry-picks the worker's squashed delta
  onto a new `review/auth-epic/01-ratchet-auth-mw`, then
  `02-moss-api-endpoints` on top of that, then
  `03-drift-frontend`. It runs
  `gh stack init --adopt review/auth-epic/01-ratchet-auth-mw review/auth-epic/02-moss-api-endpoints review/auth-epic/03-drift-frontend`
  followed by `gh stack submit --auto --draft`.
- Phase D (review + merge): humans review the stacked PRs as a ladder. When
  satisfied, the orchestrator merges the canonical workspace branch via
  `--no-ff` on `main`, closes the review PRs, and deletes `review/auth-epic/*`.
- Show actual tool calls: `dag-check`, cherry-pick, `gh stack init --adopt`,
  `gh stack submit --auto --draft`, `pr-create` (not used here because
  `gh stack submit` creates the review PRs), final `gh pr merge` on the
  canonical PR.

### 7. Risks and rejected alternatives
- **Risk: drift between canonical and review topology.** Mitigation:
  `stack-project` is deterministic over the DAG; run it late (after canonical
  integration) so it cannot desync.
- **Risk: reviewers review the wrong artifact and approve review PRs that
  never reach main.** Mitigation: label review PRs `[review-only]`, disable
  their merge button via branch protection on `review/*`, require approval
  on the canonical PR.
- **Risk: `gh stack`'s strict linearity clashes with non-linear DAGs.**
  Mitigation: when the DAG branches, emit multiple sibling stacks
  (`review/<epic>/a/*`, `review/<epic>/b/*`) and document that fan-out DAGs
  produce multiple ladders, not one.
- **Rejected alternative 1: "convention-only, no code changes."** Reuses
  `pr-create`'s `base` parameter but produces no stacked-PR UX on
  GitHub — the PRs are linked only by base, not by `gh stack`'s stack
  metadata. Loses the main reviewer benefit of gh-stack (the ladder view).
- **Rejected alternative 2: "stack-mode replaces merge-based integration."**
  Maximum reviewer benefit, but shreds the `--no-ff` audit trail LOOM
  currently relies on for `git log --first-parent main` to mean anything.
  We considered preserving audit via reflogs; reflogs are local and expire,
  so this is not a real substitute.
- **Rejected alternative 3: "workers run `gh stack` themselves."**
  Elegant from gh-stack's perspective but requires relaxing at least two
  LOOM invariants (workspace-write, no-cross-worktree). Cost is too high
  for a reviewer-UX benefit.

## Key references

- `/home/willem/.agents/skills/gh-stack/SKILL.md` — canonical gh-stack behaviour,
  especially the `init --adopt`, `submit --auto --draft`, and linearity
  limitation (line ~789).
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md`
  §6.1 (trust boundaries) and §7 (coordination) — justifies the
  orchestrator-only authority argument.
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md`
  §3.3 (assignment trailers, `Dependencies`) — shows the DAG is already
  first-class; §5.7 (orchestrator post-terminal commit template) — the
  shape `stack-project` will emit.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts`
  lines 32-45 — confirms `pr-create` already takes arbitrary `base`, so the
  review topology needs no new PR-creation primitive.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts`
  lines 27-40 — confirms retargeting is trivial via `gh pr edit --base`,
  relevant if review PRs need re-parenting after DAG changes.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts`
  lines 140-165 (Kahn's topo sort) — the exact input `stack-project` will
  consume to decide branch ordering.
- GitHub issue #74 (the epic RFP) — the five sharp edges listed in the
  issue body are the exact five objections this angle must defeat.
