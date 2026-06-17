# Team 2 — RFP Outline: Convention-Only Stacked PRs (Zero Code Changes)

## Angle statement

**Convention-only, zero code changes: stacked PRs are a documented orchestrator recipe layered on top of the existing `assign`, `dispatch`, `pr-create`, `pr-retarget`, `push`, and `commit` tools and the existing `Dependencies:` trailer — no new MCP tools, no new trailers, no schema changes, no worker-template edits.**

## Thesis

The RFP itself concedes the mechanical linchpin: *"`loom-tools` already supports custom PR bases. `pr-create` and `pr-retarget` both accept arbitrary `base` branches. A minimal-viable stacking path needs no tool changes at all."* (Issue #74, sharp edge #1.) Every other angle in this round will spend design budget reconciling audit trails, worker authority, or branch namespaces against machinery that does not yet exist. This angle spends its design budget on the opposite question: **how far can we get with what already exists, and what does that look like in practice?** The answer is *all the way* — a linear `Dependencies` DAG is already a stack, `pr-create --base` already points a PR at a sibling branch, and `pr-retarget` already relinks a PR after its predecessor merges. What is missing is not code but a written recipe the orchestrator can follow verbatim. A convention-only proposal can ship the same afternoon it is approved, leaves every LOOM invariant untouched, and preserves the option value of building heavier machinery later once this recipe has produced real evidence about what the sharp edges actually are (as opposed to what we currently suspect). Doing this with zero code changes is the right bet because it is the cheapest possible experiment that still answers the question — and in a five-team RFP where three other teams will almost certainly propose new tools or trailers, the convention-only baseline is the control group the winner selection needs.

## Section headers for `proposal.md`

Each heading below MUST appear verbatim in the writer's document. Bullets are the claims the writer MUST argue.

### 1. Angle statement

- State the convention-only thesis in one sentence, framing it as "zero new code, one new documentation page."
- Name the bet explicitly: the cheapest proposal that could possibly work is also the proposal that preserves the most option value, because nothing it ships has to be un-shipped to adopt a heavier approach later.
- Call out the one constraint the recipe accepts as a hard limit: stacks are strictly linear (matching both `gh-stack`'s own limitation and `Dependencies:` chains that form a single path).
- Name what we explicitly refuse to build this round: new MCP tools, new trailers, new schema fields, worker-template edits, new worker roles, or anything that would force `repos/bitswell/loom-tools` to cut a release.

### 2. What changes

- **`loom-tools`: nothing.** Not one line in `src/tools/pr-create.ts`, `pr-retarget.ts`, `push.ts`, `commit.ts`, `dag-check.ts`, or `src/tools/index.ts`. The existing `pr-create` handler at lines 32–42 already forwards `--base` to `gh pr create` verbatim, and `pr-retarget` at lines 30–34 already runs `gh pr edit --base`. That is the entire mechanical surface the recipe needs.
- **Loom plugin skill: exactly one new recipe page** at `plugins/loom/skills/loom/references/stacked-prs.md` (sibling of `examples.md`, `protocol.md`, `schemas.md`, `worker-template.md`). A short pointer is added to the existing `SKILL.md` "references" list so Claude finds it — that single-line edit is the full extent of the plugin change.
- **`schemas.md`: nothing.** No new trailer. No new state. No new ASSIGNED field. The recipe consumes the existing `Dependencies:` trailer (schemas.md §3.3) exactly as documented.
- **`worker-template.md`: nothing.** Workers are completely unaware that their branch is part of a stack. They read the ASSIGNED commit, commit to `loom/<agent>-<slug>`, reach `COMPLETED`, and stop — identical to today.
- **What the recipe page contains, concretely**: (a) when to use it (linear `Dependencies:` chain, 2+ agents, review ladder desired); (b) an ordering rule — topologically sort the agents by `Dependencies:` and call this the stack order; (c) the `pr-create` call for each layer with a precise `base:` argument derived from that ordering (layer 0 → `main`, layer N → `loom/<previous-agent>-<previous-slug>`); (d) the `pr-retarget` fixup to run when a lower layer merges and the next layer's base disappears; (e) an explicit "do not call `gh stack` at all" note, because the `gh-stack` extension is about local-branch topology management and is redundant once the orchestrator is already driving things serially through `pr-create`.
- **Why a recipe page is the right artefact, not a skill or a command**: recipes in the `references/` directory are already the established pattern for "composed use of existing primitives" in this plugin (see `examples.md`). Adding one more file matches existing conventions and requires no plugin machinery whatsoever.

### 3. Branch naming and scope

- Every branch stays `loom/<agent>-<slug>` exactly as `schemas.md` §2 requires. The recipe never creates branches — it only passes branch names that already exist (because `assign()` created them) as the `head:` and `base:` arguments to `pr-create`.
- `Scope:` enforcement is unchanged because the recipe touches no worker behavior. Each worker's commits are still validated against its own `Scope:` at `integrate()` time, exactly as `protocol.md` §3.3 step 2 specifies.
- No new namespace is introduced. `gh-stack`'s `-p` prefix model is not used, and `gh stack init` is not called, so there is no `feat/*`, `review/*`, or other parallel namespace to reason about.
- Collision risk with other teams' angles is zero because this proposal owns no branches at all — it only documents how to point existing branches at each other as PR bases.

### 4. Merge vs rebase

- There is no rebase in this proposal, full stop. The orchestrator still integrates each agent's branch with `git merge --no-ff` exactly as `protocol.md` §3.3 and §8.2 require; the `main` audit trail is byte-identical to a LOOM run without stacking.
- `gh-stack`'s rebase-and-force-push model never runs. The `gh stack` CLI is not invoked by any step of the recipe. The "audit-trail incompatibility" sharp edge in issue #74 is therefore moot — it only existed if someone used `gh stack`'s own rebase commands, which this recipe refuses to do.
- The key insight the writer must defend: stacked *PRs* and stacked *branches managed by the gh-stack extension* are not the same thing. GitHub has supported PRs with arbitrary bases for years; `pr-create --base loom/ratchet-auth` produces a stacked PR today, using only `gh pr create` under the hood. We are adopting the PR topology without adopting the rebase workflow.
- Honest tradeoff to name: if a lower layer is amended after its PR is created, the higher layers' PRs will show a confusing diff until they are rebased. The recipe's answer is *do not amend integrated layers* — which is already the LOOM norm, since `integrate()` is the point of no return for a worker's branch.

### 5. Worker authority

- Workers never invoke `gh stack`, `gh pr create`, `gh pr edit`, or any git command beyond the ones their current worker template already permits. Zero LOOM invariants are relaxed.
- The orchestrator is the sole caller of `pr-create` and `pr-retarget`, as today — both tools already declare `roles: ['orchestrator']` (pr-create.ts line 27, pr-retarget.ts line 25) and this angle does not touch those role annotations.
- The recipe adds exactly one new orchestrator responsibility, and it is a read-only one: topologically sort the `Dependencies:` trailers of a cohort of ASSIGNED branches before calling `pr-create` on them. No new authority point, no new write path, no new failure mode distinct from what `pr-create` already produces.
- Contrast the writer should draw: angles that let workers run `gh stack submit` from their worktrees need new scope rules for stack metadata files, new heartbeat semantics during rebase windows, and a relaxation of the "only orchestrator writes to workspace" rule in `protocol.md` §6.1. This angle needs none of that because the worker side of the protocol simply never learns that stacking is happening.

### 6. End-to-end example

The writer MUST trace this epic literally, with every `assign`, `dispatch`, `pr-create`, and `pr-retarget` call spelled out with its `base:` argument.

- **Epic**: "add auth middleware → API endpoints → frontend UI" decomposed into three agents: `ratchet/feat-auth`, `moss/feat-api` (depends on `ratchet/feat-auth`), `ratchet/feat-frontend` (depends on `moss/feat-api`). Linear `Dependencies:` chain.
- **Step 1 — orchestrator assigns all three in one pass**, using the existing `assign` tool. Each ASSIGNED commit carries the ordinary `Dependencies:` trailer per `schemas.md` §3.3:
  - `loom/ratchet-feat-auth` — `Dependencies: none`
  - `loom/moss-feat-api` — `Dependencies: ratchet/feat-auth`
  - `loom/ratchet-feat-frontend` — `Dependencies: moss/feat-api`
- **Step 2 — orchestrator dispatches `ratchet-feat-auth` only** (via `dispatch`), because the other two are blocked on their dependencies; `loom-dispatch` already handles this gate (`protocol.md` §4.1 — "Blocked assignments remain ASSIGNED until dependencies are met").
- **Step 3 — ratchet drives `feat-auth` to COMPLETED**, then the orchestrator runs:
  - `pr-create { head: "loom/ratchet-feat-auth", base: "main", title: "feat(auth): add auth middleware", body: "<auto>" }`
  - This is the **bottom of the stack**, base `main`. No other stack-specific argument is needed because `pr-create` already accepts any `base`.
- **Step 4 — orchestrator dispatches `moss-feat-api`** (now unblocked because `ratchet/feat-auth` is COMPLETED). Note: the orchestrator does *not* integrate `ratchet/feat-auth` into `main` yet — this is what makes the stack real. The branch stays open as a PR, and the next layer stacks on top of it.
- **Step 5 — moss drives `feat-api` to COMPLETED**, then the orchestrator runs:
  - `pr-create { head: "loom/moss-feat-api", base: "loom/ratchet-feat-auth", title: "feat(api): add API endpoints", body: "<auto>" }`
  - **Middle of the stack**: `base` is literally the previous layer's LOOM branch name. GitHub renders only the diff between these two branches, giving the reviewer the clean per-layer view that is the whole point of stacking.
- **Step 6 — orchestrator dispatches `ratchet-feat-frontend`**, worker drives to COMPLETED, then:
  - `pr-create { head: "loom/ratchet-feat-frontend", base: "loom/moss-feat-api", title: "feat(frontend): add frontend UI", body: "<auto>" }`
  - **Top of the stack**: `base` is the middle layer's branch. Three PRs now exist, each showing a single layer's diff.
- **Step 7 — reviewer merges bottom-up**. When PR #1 (`ratchet-feat-auth` → `main`) merges, `loom/ratchet-feat-auth` disappears from GitHub, which would orphan PR #2's base. The orchestrator runs:
  - `pr-retarget { number: <pr2>, base: "main" }`
  - This is the exact case the existing `pr-retarget` tool was built for; its handler at `pr-retarget.ts` lines 30–34 is literally `gh pr edit <number> --base <base>`. No new code path is exercised.
- **Step 8 — repeat** as PR #2 merges: `pr-retarget { number: <pr3>, base: "main" }`.
- **Step 9 — epic complete**. `main` has three `--no-ff` merge commits in order. The audit trail is identical to a non-stacked LOOM run. No branch was rebased, no history was rewritten, no worker saw any stack machinery. The writer should end by noting that the *total* new tool-call surface area introduced by this recipe is: three `pr-create` calls and two `pr-retarget` calls — all against tools that already exist and are already role-gated.

### 7. Risks and rejected alternatives

- **Risk — linear-only stacks**: the recipe only works when `Dependencies:` forms a single chain. Diamond or fan-in DAGs cannot be rendered as a `gh-stack`-style ladder. The writer should name this honestly and point out that `gh-stack` itself has the same limitation (SKILL.md "Known limitations" #1), so the recipe is not ceding ground the tool would have given us.
- **Risk — base-branch deletion on merge**: when a lower PR merges, its branch is deleted and higher PRs' bases go stale until retargeted. The mitigation is the `pr-retarget` step in the recipe (§6 step 7); the residual risk is that the orchestrator forgets to run it, which manifests as a confused-looking PR but no data loss.
- **Risk — drift during long review cycles**: if a reviewer sits on the stack for days while `main` advances, the bottom PR's merge will produce merge commits that higher layers have not seen. Because LOOM uses `--no-ff` merges and does not rebase, this manifests as a merge conflict at the time of the next integrate, resolved the same way any other conflict is (`protocol.md` §5.2 `conflict` category). No new failure mode.
- **Risk — reviewers expecting gh-stack UX**: a human reviewer familiar with `gh stack view --json` will find nothing to view, because no `gh stack` state exists locally. The writer should note this is fine — GitHub's native stacked-PR UI renders correctly from the PR `base` chain alone.
- **Rejected alternative 1 — "add a `Stack-Position:` trailer to ASSIGNED commits"**: would duplicate information already encoded in `Dependencies:`, would force every team (even non-stacked ones) to accept a new required field, and would grow `schemas.md` §3.3 for no mechanical benefit. Rejected because any information the orchestrator needs can be derived from `Dependencies:` at recipe-execution time.
- **Rejected alternative 2 — "add a new `stack-create` MCP tool that wraps the recipe"**: would move the recipe from a documentation page into executable code, which locks in the recipe before we have evidence it is the right shape and forces a `loom-tools` release cycle for every tweak. Rejected because the whole point of this angle is to treat the recipe as a hypothesis, not a commitment. If the recipe proves correct after real use, promoting it to a tool is a one-file PR later.
- **Rejected alternative 3 — "invoke `gh stack init --adopt` and `gh stack submit --auto`"**: would introduce rebase-and-force-push semantics on loom branches, violating `protocol.md` §8.2 and requiring new scope rules. Rejected because it re-opens every sharp edge the recipe was designed to sidestep, in exchange for no benefit that `pr-create --base` does not already deliver.

## Key references

- **Issue #74 (the RFP itself)** — sharp edge #1 ("`loom-tools` already supports custom PR bases"). This is the explicit invitation for a zero-code proposal and must be quoted in §1 of the writer's document.
- **`pr-create.ts`** — `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts` lines 6–11 (`PrCreateInput` schema, including the `base: z.string()` field with no restriction on its value) and lines 32–42 (handler that forwards `--base` verbatim to `gh pr create`). Proves the load-bearing claim: stacking needs no code changes.
- **`pr-retarget.ts`** — `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts` lines 6–9 (schema) and lines 30–34 (handler that runs `gh pr edit <number> --base <base>`). Proves the fixup step is a single existing tool call.
- **`pr-create.ts` line 27 and `pr-retarget.ts` line 25** — both `roles: ['orchestrator']`. Proves worker authority is already correctly gated; the recipe inherits this gating for free.
- **`schemas.md` §3.3** — `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` lines 72–82, the `Dependencies:` trailer definition. This is the entire information source the recipe reads; it needs nothing else.
- **`protocol.md` §3.3 `integrate()`** — `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` lines 86–98. Establishes that `--no-ff` merge is the integration primitive; the recipe preserves this unchanged.
- **`protocol.md` §8.2 audit trail** — lines 182–184. The invariant the recipe preserves by refusing to rebase; the writer should quote it in §4.
- **`protocol.md` §4.1 `loom-dispatch`** — lines 104–114. Establishes that dependency gating is already automatic, so the recipe does not need to re-implement "wait for lower layer to finish."
- **`protocol.md` §6.1 trust boundary** — lines 157–165. The table the recipe does not touch: workspace write stays orchestrator-only, worker scope stays per-branch.
- **`schemas.md` §2 branch naming** — lines 34–52. The `loom/<agent>-<slug>` pattern the recipe uses as `head:` and `base:` arguments verbatim; no new namespace introduced.
- **`gh-stack` SKILL.md "Known limitations" #1** — `/home/willem/.agents/skills/gh-stack/SKILL.md` line 789 ("Stacks are strictly linear"). Justifies accepting the same constraint in the recipe — this is a limitation of the stacked-PR model itself, not of the convention-only approach.
- **Loom plugin skill layout** — `/home/willem/bitswell/bitswell/repos/bitswell/loom-plugin/plugins/loom/skills/loom/references/` already contains `examples.md`, `protocol.md`, `schemas.md`, `worker-template.md`. The new `stacked-prs.md` recipe page sits alongside them as a sibling; this is the exact location the writer must cite when describing the one-file delta.
- **`examples.md` in the loom plugin skill** — establishes the precedent that composed workflows ("assign, then dispatch, then integrate") are documented as narrative recipes in the skill, not as new tools. The recipe this proposal adds follows that precedent precisely.
