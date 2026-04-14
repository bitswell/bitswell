# Team 2 — RFP Outline: gh-stack as a Post-Integration Review Projection

## Angle statement

**Review-projection mode: LOOM integrates normally with `--no-ff` merges, then the orchestrator projects the already-merged DAG as a read-only `gh stack` ladder on throwaway `review/*` branches — the stack is a reviewer view, not a branching model.**

(29 words.)

## Thesis

LOOM's audit-trail invariant and `gh-stack`'s rebase-and-force-push model are genuinely incompatible; every other integration angle either sacrifices the `--no-ff` log, relaxes worker authority, or grows a parallel branching convention that duplicates the `Dependencies` DAG. This proposal severs the review UI from the source-of-truth branching: the canonical history stays exactly as LOOM produces it today, and `gh stack` runs *after* integration on disposable mirror branches whose only purpose is rendering a reviewable ladder in the GitHub UI. Stacked PRs become an output format, not an input format — which lets us ship in days, preserves every LOOM invariant, and keeps workers untouched. The cost we honestly own: the stacked PRs are advisory, not the merge path, so the RFP reader must accept that "merge the stack" ≠ "click merge in GitHub."

## Section headers for `proposal.md`

Each heading below must appear verbatim in the writer's document. Bullets under each heading are the claims the writer MUST argue.

### 1. Angle statement
- State the review-projection thesis in one sentence.
- Frame the key inversion: source-of-truth branches are LOOM's; stacked PRs are a *derived artifact*.
- Name the invariant we refuse to break: `--no-ff` audit trail on `main`.
- Name what we explicitly give up: one-click merge from the GitHub stack UI.

### 2. What changes
- **`loom-tools`**: add exactly one new orchestrator-only tool, `stack-project` (roles: `['orchestrator']`), which consumes a DAG (same shape as `dag-check.ts` input) and emits a `gh stack init --adopt` + `gh stack submit --auto --draft` sequence against a mirror namespace.
- **No changes** to `pr-create.ts`, `pr-retarget.ts`, `commit.ts`, `push.ts`, `dag-check.ts`, or any schema; `pr-create` already accepts an arbitrary `base` and that is all we need for the real merge path.
- **`loom` skill**: add one new recipe page (`references/review-projection.md`) documenting when to run the projection; no changes to the worker template.
- **No new trailers, no new states, no changes to `mcagent-spec.md`.** The projection reads only the existing `Dependencies:` trailer and the integrated merge graph.
- File path specificity: new code lives at `repos/bitswell/loom-tools/src/tools/stack-project.ts` registered in `src/tools/index.ts`, reusing the `dag-check` adjacency logic.

### 3. Branch naming and scope
- LOOM keeps `loom/<agent>-<slug>` as the only authoritative namespace; `gh-stack`'s prefix model is applied to a disjoint `review/<epic-slug>/<layer>` namespace owned exclusively by the orchestrator.
- `Scope:` enforcement is unchanged because workers never see `review/*` branches — they commit only inside `loom/<agent>-<slug>` worktrees as today.
- The projection step happens after `integrate()` (protocol §3.3), so scope validation has already run against the original branch; `review/*` commits are mechanical fast-forwards of orchestrator-authored merge commits and contain no scope-bearing trailers.
- Collision risk with worker branches is zero because `review/` and `loom/` namespaces cannot overlap; `gh stack init -p review/<slug>` is safe by construction.

### 4. Merge vs rebase
- The real merge path stays `--no-ff` into `main`, authored by the orchestrator, exactly as the current protocol mandates — the audit trail is untouched.
- `gh stack`'s rebase-and-force-push happens only on `review/*` branches, which are regenerated from scratch on every epic and have no downstream consumers, so force-push is harmless.
- We argue that the RFP's "audit-log incompatibility" is a false dichotomy: it only exists if you let gh-stack own the merge path. Move gh-stack to the review path and the incompatibility disappears.
- Trace one concrete example showing the `main` branch log is byte-identical to a LOOM run without gh-stack integration, proving the projection is non-invasive.

### 5. Worker authority
- Workers never run any `gh stack` command — ever. The `roles: ['orchestrator']` annotation (already used by `pr-create.ts:27` and `pr-retarget.ts:25`) gates the new `stack-project` tool the same way.
- No LOOM invariant is relaxed: workers still commit only to their own `loom/<agent>-<slug>` branch, still cannot create PRs, still cannot write to workspace.
- The orchestrator runs the projection as a single tool call post-integration; this is one new authority point, not a distributed one — easier to reason about and to audit.
- Contrast explicitly with angles that let workers run `gh stack submit` from their worktrees: those require new scope rules, new heartbeat semantics for in-flight rebases, and new failure-mode handling we simply don't need.

### 6. End-to-end example
- Epic: "add auth middleware → API endpoints → frontend UI" with three workers `ratchet/auth`, `moss/api`, `ratchet/ui` and `Dependencies:` forming a linear DAG.
- Phase A (unchanged): orchestrator assigns three tasks on `loom/ratchet-auth`, `loom/moss-api`, `loom/ratchet-ui`; workers commit `IMPLEMENTING → COMPLETED`; orchestrator integrates each into `main` with `git merge --no-ff` in topo order from `dag-check`.
- Phase B (new): orchestrator calls `stack-project` with the same DAG. Tool creates `review/auth-feature/auth`, `review/auth-feature/api`, `review/auth-feature/ui` via `gh stack init --adopt`, where each mirror branch points at the corresponding merge commit SHA on `main`.
- Phase C: tool runs `gh stack submit --auto --draft` (drafts, because nobody should click merge on them); three stacked PRs appear in GitHub review UI, each showing only its layer's diff.
- Phase D: humans review the stack. Nothing merges from the stack — the real merges already happened. When review completes, orchestrator deletes `review/auth-feature/*`.
- Walk the exact tool call sequence with JSON inputs, including the `roles` check and the heartbeat-free nature of post-integration projection.

### 7. Risks and rejected alternatives
- **Risk — "two PRs per change" UX**: every sub-task produces a real (merged) PR from `pr-create` *and* a draft review-projection PR. Mitigate by closing the real PRs as "superseded by review stack" or by making the real merges silent (no PR, direct merge commit), which the RFP's non-goals permit.
- **Risk — reviewers want to click merge**: the stack is advisory. Mitigate with a PR body template that explains the projection and a bot comment linking to the already-merged SHA.
- **Risk — projection drift**: if someone hand-edits `main` between integrate and project, the stack misrepresents the DAG. Mitigate by running projection inside the same orchestrator transaction as the last integrate.
- **Rejected alternative 1 — "Workers run `gh stack submit` from their worktrees"**: relaxes worker authority boundary, needs new scope rules for `.git/refs/stack-*`, introduces heartbeat gaps during rebase, and duplicates `Dependencies` with `gh-stack`'s own metadata. Rejected as expensive and duplicative.
- **Rejected alternative 2 — "Replace `--no-ff` merges with gh-stack rebases on `main`"**: directly breaks the audit-trail invariant in protocol.md §8.2. Rejected as protocol-violating.
- **Rejected alternative 3 — "New `Stack-Position:` trailer + schema extension"**: forces every team to adopt a new trailer even for non-stacked work, grows `schemas.md` §3, and still does not solve the merge/rebase incompatibility. Rejected as ceremony without payoff.

## Key references

- **gh-stack skill** — `/home/willem/.agents/skills/gh-stack/SKILL.md` (agent rules, especially the non-interactive-flags section and the `--adopt` flag for existing branches — this is what makes the projection possible).
- **LOOM protocol §3.3 `integrate()`** — `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` lines 86–98 (defines the exact seam where the projection hooks in: after merge-into-workspace, before the next assignment).
- **LOOM protocol §6.1 trust boundary** — same file, lines 157–165 (the table the proposal must not violate: workspace write = orchestrator only).
- **LOOM schemas §2 branch naming** — `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` lines 34–52 (the `loom/<agent>-<slug>` pattern the `review/` namespace explicitly avoids colliding with).
- **`pr-create.ts`** — `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts` lines 21–28 (the `roles: ['orchestrator']` pattern the new tool copies verbatim; also proves arbitrary `base` is already supported, which matters for angle comparisons).
- **`pr-retarget.ts`** — `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts` lines 19–41 (shows `gh pr edit --base` is the retargeting primitive; the writer should note this is NOT needed for this angle, but IS needed by competitor angles — a point of contrast).
- **`dag-check.ts`** — `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts` lines 79–201 (the adjacency-list + Kahn's topo sort that `stack-project` reuses unchanged to order the review layers).
- **Protocol §8.2 audit trail** — `protocol.md` lines 182–184 (the invariant this proposal refuses to break — quote it in §4 of the writer's doc).
- **`gh stack init --adopt`** — documented in SKILL.md quick-reference table (the specific flag that lets the orchestrator adopt already-merged branches into a stack without rewriting history; this is the mechanical linchpin of the whole approach).
