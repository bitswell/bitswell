# Team 1 — OUTLINE

## Angle statement

Stacks are a **read-only presentation layer**: LOOM keeps merging as today, and the orchestrator projects a shadow `stack/*` mirror namespace via `gh-stack` purely for reviewer UX.

## Thesis

The "merge vs rebase" and "worker authority" sharp edges are only real if we try to make `gh-stack` the *integration mechanism*. They disappear if `gh-stack` is instead an **output artifact** — a second, disposable view of the same DAG, rebuilt on demand from the existing `loom/<agent>-<slug>` branches. LOOM keeps its `--no-ff` audit trail untouched, workers keep their no-PR invariant, and reviewers still get the ladder-of-diffs UX that motivates this RFP. The cost is a narrow orchestrator recipe plus one new tool (`stack-project`), not a protocol rewrite. This angle is the bet that stacks are worth their UX value but not worth paying for with LOOM's audit invariants.

## Section headers for proposal.md

### 1. Angle statement

- State the read-only-projection bias in one sentence and contrast with "gh-stack as integration mechanism"
- Declare the invariant this protects: LOOM's `--no-ff` merge audit trail stays untouched
- Name the explicit tradeoff: stack PRs are disposable artifacts, not the canonical history
- Frame `gh-stack` as an *output*, not an *input*, of the LOOM pipeline

### 2. What changes

- One new `loom-tools` tool: `stack-project` (orchestrator-only), implemented as a thin wrapper over `gh stack init --adopt` + `gh stack submit --auto --draft`, living next to `pr-create.ts`
- Zero changes to `pr-create.ts`, `pr-retarget.ts`, `commit.ts`, `push.ts` — the existing PR tools continue to operate on `loom/*` branches
- One new orchestrator recipe in the `loom` plugin: "project current DAG as a stack" (invoked post-plan-gate, before per-agent PRs are opened)
- Zero changes to `schemas.md`, worker template, or the trailer vocabulary — no new `Stack-*` trailers
- New shadow namespace convention documented in `protocol.md` §7: `stack/<epic-slug>/<layer-n>` for mirror branches

### 3. Branch naming and scope

- `loom/<agent>-<slug>` branches remain authoritative and scope-enforced exactly as today; no stacking logic reads them
- Mirror branches live under `stack/<epic-slug>/<layer-n>`; they are regenerated from scratch on every projection and never receive human commits
- `Scope:` enforcement is unchanged — it runs against the `loom/*` worktree at integration time; mirror branches have no `Scope:` because they contain no original work
- The `stack/*` namespace is explicitly excluded from `loom-dispatch --scan` to prevent accidental worker spawns
- Deletion policy: on successful merge of the last layer, all `stack/<epic-slug>/*` branches are pruned by the orchestrator

### 4. Merge vs rebase

- LOOM integration stays `--no-ff` merge into workspace, unchanged
- `gh-stack`'s rebase+force-push model is confined to the `stack/*` mirror namespace, where force-pushing is safe by definition (the branches are regenerated)
- The audit trail lives in the workspace and the `loom/*` branches; the stack PRs are a *view* and are not themselves audited
- Concretely: when a reviewer approves `stack/epic-slug/layer-2`, the orchestrator does **not** merge that PR — it closes the stack PR set and instead runs the existing per-`loom/*`-branch integration path
- The projection is idempotent and can be rebuilt after every upstream change without rewriting history

### 5. Worker authority

- Workers invoke zero `gh stack` commands; they remain forbidden from calling any `gh pr` or `gh stack` subcommand
- Only the orchestrator invokes `stack-project`, and only from the workspace, never inside a worktree
- The existing invariant "only bitswell writes to the workspace" extends naturally: "only bitswell publishes stacks"
- No LOOM invariant is relaxed; no worker template changes
- Rationale: giving workers PR authority is the load-bearing assumption in most `gh-stack` integrations, and it's the one that breaks LOOM's security model hardest

### 6. End-to-end example

- 3-agent epic: `auth-middleware → api-endpoints → frontend` with `Dependencies: auth-middleware`, `Dependencies: api-endpoints` declared in ASSIGNED commits
- Phase 1: orchestrator runs `dag-check` → topological sort `[auth, api, frontend]`, spawns all three workers in parallel worktrees
- Phase 2: workers commit to `loom/ratchet-auth-middleware`, `loom/moss-api-endpoints`, `loom/ratchet-frontend` with COMPLETED; integration merges them into workspace in DAG order via existing `--no-ff` flow
- Phase 3: orchestrator invokes `stack-project --epic add-auth --order auth,api,frontend`; the tool creates `stack/add-auth/01-auth`, `stack/add-auth/02-api`, `stack/add-auth/03-frontend` by cherry-picking the integrated merges, then runs `gh stack init --adopt ... && gh stack submit --auto --draft`
- Phase 4: reviewers see a three-layer stack of draft PRs, walk it, leave comments; any requested change goes back to the original `loom/*` branch via a re-dispatched worker; `stack-project` reruns and force-updates the mirror
- Phase 5: on final approval, orchestrator closes the stack PRs, opens a single `--no-ff` merge PR for the entire epic (or the five per-agent PRs, per existing policy), and prunes `stack/add-auth/*`

### 7. Risks and rejected alternatives

- **Risk: reviewer confusion.** Stack PRs are draft and don't merge; reviewers must know approval flows back through `loom/*`. Mitigation: PR body template explaining the projection model, plus a `[MIRROR]` prefix on titles
- **Risk: divergence.** If the mirror drifts from the `loom/*` source (e.g., a worker amends after projection), the stack view lies. Mitigation: `stack-project` is idempotent and cheap; re-run on every upstream change; add a freshness check comparing `loom/*` HEADs to `stack/*` cherry-pick sources
- **Risk: cherry-pick conflicts when projecting.** Mitigation: project in DAG order, reuse `git rerere` cache, fail the projection cleanly rather than half-publishing
- **Rejected alternative 1: "give workers `gh stack` authority."** Fastest path to real stacks, but breaks the "only orchestrator touches GitHub PRs" invariant and fragments the audit trail across rebased branches. Not worth the blast radius
- **Rejected alternative 2: "replace `--no-ff` integration with `gh stack sync`."** Would give stacks first-class status but destroys the merge-based audit log and makes cross-agent rebase conflicts load-bearing. LOOM's debuggability depends on a linear-mergy workspace history
- **Rejected alternative 3: "new `Stack-Parent` / `Stack-Order` trailers on worker commits."** Adds protocol surface area without removing any existing surface; the DAG already encodes the order via `Dependencies`, so new trailers are redundant

## Key references

- `/home/willem/.agents/skills/gh-stack/SKILL.md` — canonical `gh-stack` reference; note especially the "Restructure a stack" workflow (`gh stack unstack` + `init --adopt`) which is the technical basis for the projection approach
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` §6.1 "Trust boundary" — the invariant this proposal preserves
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` §3.3 "Assignment trailers" and §3 "Trailer Vocabulary" — confirm no new trailers needed
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts` — reference implementation for a new orchestrator-role `loom-tools` tool (shows role enforcement and `exec` pattern); `stack-project` should mirror this shape
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts` — proves the `orchestrator`-role convention already exists in the codebase
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts` — the topological sort the projection must consume as input (`integrationOrder` field)
- GitHub issue `bitswell/bitswell#74` — the full RFP spec, especially "Sharp edges" items 4 (merge vs rebase) and 5 (worker authority) which this angle targets
