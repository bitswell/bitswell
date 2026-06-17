# Team 2 — Convention-Only Stacked PRs: Zero Code Changes, One Recipe Page

**Epic**: bitswell/bitswell#74 — "RFP: Integrate gh-stack into the LOOM Protocol"
**Team angle**: convention-only, zero code changes
**Status**: proposal (design-only round; no code lands in this PR)

---

## 1. Angle statement

**Convention-only, zero code changes: stacked PRs are a documented orchestrator recipe layered on top of the existing `assign`, `dispatch`, `pr-create`, `pr-retarget`, `push`, and `commit` tools and the existing `Dependencies:` trailer — no new MCP tools, no new trailers, no schema changes, no worker-template edits.**

This proposal ships one new documentation page and nothing else. The bet it names explicitly: the cheapest proposal that could possibly work is also the proposal that preserves the most option value, because nothing it ships has to be un-shipped to adopt a heavier approach later. If convention-only turns out to be the wrong answer after real usage, we promote the recipe to a tool in a one-file PR. If it turns out to be the right answer, we have already shipped it.

The one constraint this angle accepts as a hard limit is that stacks are strictly linear. A single path through the `Dependencies:` DAG is treated as a single stack; diamond or fan-in shapes fall back to the current non-stacked LOOM flow. This is not a concession — it is the same limitation `gh-stack` itself documents in its SKILL.md Known Limitations list: "Stacks are strictly linear. Branching stacks (multiple children on a single parent) are not supported." (`/home/willem/.agents/skills/gh-stack/SKILL.md` line 789.) Accepting the same limitation as the tool we are trying to interoperate with is not ceding ground.

What this proposal explicitly refuses to build this round:

- No new MCP tools in `repos/bitswell/loom-tools/src/tools/`.
- No new commit trailers in `schemas.md` §3.
- No new required fields on ASSIGNED commits in `schemas.md` §3.3.
- No changes to `worker-template.md` or to any worker role.
- No new worker roles, no new orchestrator sub-roles.
- No release of `repos/bitswell/loom-tools`.
- No invocation of `gh stack` subcommands anywhere in the recipe.

What it does ship:

- One new file: `plugins/loom/skills/loom/references/stacked-prs.md`, a recipe page written in the same narrative style as the existing `examples.md`.
- One single-line addition to the existing `SKILL.md` references list so Claude-Code's skill loader can find the new page.

Total delta: two files, one of them new, the other edited by one line. Zero lines of TypeScript.

---

## 2. Thesis

The RFP itself concedes the mechanical linchpin in its "Sharp edges" section:

> "`loom-tools` already supports custom PR bases. `pr-create` and `pr-retarget` both accept arbitrary `base` branches. **A minimal-viable stacking path needs no tool changes at all.**"
> — Issue #74, sharp edge #1 (emphasis added)

Every other angle in this five-team round will spend design budget reconciling audit trails, worker authority, or branch namespaces against machinery that does not yet exist. Team 1 (first-class protocol extension) will introduce a new `Stack-Position:` trailer; team 3 (adopt `gh-stack` wholesale) will have to relax the worker-authority rule to let workers run `gh stack submit`; teams 4 and 5 will find their own places to pay a design tax on new state. Every one of those proposals is a commitment to a shape — a set of lines of code that, once merged, is expensive to un-merge.

This proposal spends its design budget on the opposite question: **how far can we get with what already exists, and what does that look like in practice?** The answer is surprising, and it is the single load-bearing claim of this document: *all the way*. A linear `Dependencies:` DAG is already a stack. `pr-create --base loom/ratchet-feat-auth` already points a PR at a sibling LOOM branch. `pr-retarget` already relinks a stranded PR when its predecessor merges. The orchestrator's existing responsibility to call `pr-create` at the end of each worker's lifecycle already sits at exactly the right point in the flow to inject stacking. What is missing is not code but a written recipe the orchestrator can follow verbatim.

A convention-only proposal can ship the same afternoon it is approved. It leaves every LOOM invariant untouched. It preserves the option value of building heavier machinery later, once the recipe has produced real evidence about what the sharp edges actually are — as opposed to what we currently only *suspect* they are. In a five-team RFP, the convention-only baseline is the control group the winner-selection pass needs: if a heavier angle wins, we want that to be because it demonstrably solves a problem the recipe could not. Without this baseline, the selection pass has to take every heavier angle's claimed problem statement on faith.

The proposal is also honest about the cost of being conservative. It does not deliver a prettier developer experience. It does not hide gh-stack's limitations behind a friendlier abstraction. It does not anticipate problems that might emerge at scale. What it delivers is a working end-to-end recipe, expressed in terms of tool calls that already exist, with a vocabulary the orchestrator already speaks. A writer who wanted to claim more than that would have to invent machinery, and inventing machinery at RFP time is exactly the failure mode this angle is designed to avoid.

---

## 3. What changes

This section enumerates every component that *could* change in a gh-stack integration and states, for each one, whether this proposal changes it. The bias is explicit: the default answer is "no changes." Each "no changes" is a load-bearing claim, not a hand-wave.

### 3.1 `loom-tools` — no changes

No file in `repos/bitswell/loom-tools/` is modified.

Specifically, no changes to:

- **`src/tools/pr-create.ts`.** The existing handler at lines 32–42 already forwards `--base` verbatim to `gh pr create`:

  ```ts
  const args = [
    'pr', 'create',
    '--head', input.head,
    '--base', input.base,
    '--title', input.title,
  ];
  ```

  The input schema at lines 6–11 declares `base: z.string()` with no validation narrowing its value to `main`, `master`, or any other trunk. Pointing `base` at `loom/ratchet-feat-auth` is as legal today as pointing it at `main`; the tool does not care. The whole mechanical basis of this proposal rides on these six lines continuing to do exactly what they already do.

- **`src/tools/pr-retarget.ts`.** The existing handler at lines 30–34 runs `gh pr edit <number> --base <base>`:

  ```ts
  const result = await exec(
    'gh',
    ['pr', 'edit', String(input.number), '--base', input.base],
    cwd,
  );
  ```

  This is literally the fixup step we need when a lower layer merges and the next layer's base disappears. It already exists, it is already role-gated, and it already returns a structured result the orchestrator can log in its audit trail.

- **`src/tools/push.ts`, `src/tools/commit.ts`.** Not touched. Workers still `commit` and `push` exactly as they do today. A worker does not know its branch is part of a stack.

- **`src/tools/dag-check.ts`.** Not touched. The DAG validator continues to enforce that `Dependencies:` forms a valid DAG. The recipe reads the output of a DAG walk at runtime but does not modify the walker.

- **`src/tools/index.ts`.** No new tool exports, no new names registered.

- **`src/tools/assign.ts`** (or wherever assignment lives). The orchestrator uses the existing `assign()` call with the existing `Dependencies:` trailer. Nothing new enters the assignment commit.

- **No release of `loom-tools`.** Because no source file changes, there is no version bump, no package rebuild, no installed-plugin-cache refresh, no downstream projects to notify. The tooling stays exactly where it sat yesterday.

**Role gating, inherited for free.** `pr-create.ts` line 27 and `pr-retarget.ts` line 25 both declare `roles: ['orchestrator']`. Workers already cannot call these tools. That is true today and it stays true after this proposal lands. Section 6 below walks the call sites; every `pr-create` and `pr-retarget` in the example is inside the orchestrator's invocation loop, never inside a worker's.

### 3.2 Loom plugin skill — one new file, one single-line edit

Exactly one new page is added to the plugin skill, sitting alongside the four existing reference pages at `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/`:

- `examples.md` — existing
- `protocol.md` — existing
- `schemas.md` — existing
- `worker-template.md` — existing
- **`stacked-prs.md`** — new (this proposal)

The source-of-truth copy of the file lives in the plugin repository at `plugins/loom/skills/loom/references/stacked-prs.md` and is then vended through the usual plugin-release pipeline to `/home/willem/.claude/plugins/cache/...` on install.

The new page follows exactly the same narrative recipe pattern as `examples.md`. It does not introduce any new machinery and makes no claims the tools it references cannot deliver today. Its content, concretely:

1. **When to use it.** Linear `Dependencies:` chain of two or more agents where the reviewer benefits from a per-layer diff. If the DAG forks, use the non-stacked flow. If there is only one agent, there is no stack.
2. **Ordering rule.** Topologically sort the agents by `Dependencies:` and call that the stack order. The first agent is the bottom; the last is the top.
3. **Stack-building loop.** For each layer in order, after its branch reaches `COMPLETED`, call `pr-create` with:
   - `head: "loom/<this-agent>-<this-slug>"`
   - `base: "loom/<previous-agent>-<previous-slug>"` (or `"main"` for layer 0)
   - `title:` derived from the worker's `Key-Finding` or assignment slug
   - `body:` auto-generated from the ASSIGNED commit body, as today
4. **Base-branch fixup.** After a lower-layer PR merges and its branch is deleted from the remote, run `pr-retarget` on the immediately-higher PR with `base: "main"` (or, for a middle layer still ahead of another unmerged layer, the next-lowest still-open LOOM branch).
5. **Explicit "do not call `gh stack`" note.** The `gh-stack` extension is about *local-branch topology management* — rebasing, force-pushing, stack-level metadata files. Once the orchestrator is already driving things serially through `pr-create`, `gh stack` is redundant at best and actively destructive at worst (its rebase model collides with LOOM's `--no-ff` audit trail; see §5 below). The recipe's section on this is exactly two sentences long: do not invoke `gh stack`; if you find yourself wanting to, file a follow-up RFP.
6. **Worked example.** A self-contained 3-agent walk-through identical in structure to §6 below, so a reader who lands on `stacked-prs.md` directly has everything they need.

The single-line edit to `SKILL.md` is the addition of `stacked-prs` to the references list, so Claude-Code's skill loader indexes the page on skill load. No structural change to `SKILL.md`; the new page joins the existing four references as a peer.

**Why a recipe page and not a skill, slash-command, or new tool.** The `references/` directory is the established location for "composed use of existing primitives" in this plugin. `examples.md` already documents the `assign → dispatch → integrate` flow as a narrative recipe. Adding `stacked-prs.md` as a sibling follows precedent precisely and requires no plugin-machinery changes, no new skill metadata, no new command registration. The only cost is an additional page in the skill's context when the skill is loaded; at ~300–500 lines of markdown, that cost is negligible next to what the skill already loads.

### 3.3 `schemas.md` — no changes

No new trailer. No new state. No new ASSIGNED field. No new extraction query.

The recipe consumes the existing `Dependencies:` trailer (`schemas.md` §3.3) exactly as documented today. Its format ("comma-separated `<agent>/<slug>` refs or `none`") is exactly sufficient to encode a linear chain; the ordering is recoverable by a single topological sort. Nothing in the recipe needs to know anything about the stack that is not already in the `Dependencies:` trailer.

A tempting addition — `Stack-Position: 2/3` or equivalent — is rejected in §8 below as redundant.

### 3.4 `worker-template.md` — no changes

Workers are completely unaware that their branch is part of a stack. They:

1. Read the ASSIGNED commit from their branch for the task spec.
2. Commit to `loom/<agent>-<slug>` with the required trailers.
3. Reach `Task-Status: COMPLETED`.
4. Stop.

This is identical to the current worker lifecycle (`protocol.md` §3.2 and §3.3 steps 1–2). The worker template does not need a single word changed; it does not even need to know stacking exists.

This is the single biggest advantage of the convention-only angle: the worker side of the protocol is *completely unchanged*, which means every existing worker agent (`ratchet`, `moss`, and future ones) picks up stacking capability with zero re-training, zero prompt edits, and zero chance of a worker regressing its own behavior because of a stacking-related change.

### 3.5 `protocol.md` — no changes

No state added to the lifecycle state machine. No new operation added to §3 ("Operations"). No new role defined in §1. The trust boundary table in §6.1 is byte-identical after this proposal lands.

### 3.6 `mcagent-spec.md` — no changes

Agent conformance rules are untouched because nothing about agent behavior changes.

### 3.7 Claude-Code `.claude/agents/` — no changes

No agent definition (`bitswell.md`, `ratchet.md`, etc.) is modified. The orchestrator reads a new recipe page on startup because the skill links it in, but that is the loader's job, not the agent definition's.

### 3.8 Summary delta

| Component | Changes |
|-----------|---------|
| `loom-tools` source | none |
| `loom-tools` binary release | none |
| `schemas.md` | none |
| `protocol.md` | none |
| `worker-template.md` | none |
| `mcagent-spec.md` | none |
| `examples.md` | none |
| Plugin `SKILL.md` | one line added to references list |
| Plugin references dir | one new file: `stacked-prs.md` |
| `.claude/agents/*.md` | none |

**Total files touched**: two. **Total lines of code changed**: zero. **Total lines of documentation added**: ~300–500 (the new recipe page). **Total lines of documentation edited**: one.

---

## 4. Branch naming and scope

### 4.1 Branch naming

Every branch stays `loom/<agent>-<slug>` exactly as `schemas.md` §2 requires. The constraints quoted verbatim from that section:

> - Kebab-case: `[a-z0-9]+(-[a-z0-9]+)*`
> - Maximum length: 63 characters
> - One agent per branch, one branch per assignment

This proposal does not create a single new branch name. It does not introduce a parallel namespace. It does not add a stack-name prefix. It does not touch `gh-stack`'s `-p` prefix model at all. The recipe *only* reads existing branch names (created by the existing `assign()` flow) and passes them as the `head:` and `base:` string arguments to `pr-create`.

A concrete consequence: when the orchestrator calls `pr-create { head: "loom/moss-feat-api", base: "loom/ratchet-feat-auth" }`, GitHub receives two branch names it already knows about (because both branches have been pushed via the existing `push` tool). No alias, no side-band metadata, no stack-id. The recipe is, mechanically, nothing more than a disciplined choice of `base` values.

**Collision risk with other teams' angles.** Zero. This proposal owns no branches at all — it only documents how to point existing LOOM branches at each other as PR bases. If a later round adopts a heavier angle that introduces a `stack/*` namespace, this proposal's recipe survives unchanged (the `loom/*` branches it references still exist) and can be deprecated gracefully.

### 4.2 Scope enforcement

`Scope:` enforcement is unchanged because this proposal does not touch worker behavior. Per `protocol.md` §3.3 step 2 ("Verify all files changed are within the agent's `Scope`"), the orchestrator validates each worker's commits against its own `Scope` trailer at `integrate()` time. This continues to work exactly as today.

Critically, `Scope:` is per-agent, per-branch, and per-assignment. It is not a stack-level construct. The orchestrator does not need to synthesise a "stack-wide" scope because scopes never need to be intersected across a stack — each layer is a distinct worker with its own distinct scope, and the recipe does not introduce any shared files across layers.

If the user's concern is "can layer N edit files that layer N-1 also edits?", the answer is: only if both layers' `Scope:` trailers allow the same paths, which is already the orchestrator's responsibility at assignment time, which is already how LOOM handles overlap today. Nothing about that flow changes.

### 4.3 Why not `gh stack`'s prefix model

`gh-stack`'s `init -p <prefix>` model creates branches like `feat/auth`, `feat/api`, `feat/frontend` under a single prefix. This is a useful UX for a human running `gh stack add` interactively, but it is pointless for an orchestrator that is already generating fully-qualified branch names (`loom/<agent>-<slug>`) at `assign()` time.

Using `gh stack -p loom` would create branches named `loom/feat-auth`, `loom/feat-api`, etc. — which collide catastrophically with LOOM's own `loom/<agent>-<slug>` namespace, because the branch names would no longer encode the agent that owns them. The recipe refuses to touch the prefix model at all; this is one of the mechanisms by which zero-code convention-only stacking survives contact with existing LOOM invariants.

---

## 5. Merge vs rebase

### 5.1 The recipe preserves `--no-ff` exactly

There is no rebase in this proposal, full stop. The orchestrator still integrates each layer's branch with `git merge --no-ff` exactly as `protocol.md` §3.3 ("Attempt merge") and §8.2 ("Audit trail") require. The `main` audit trail after a three-agent stack merges is byte-identical to the audit trail of a three-agent non-stacked LOOM run.

From `protocol.md` §8.2 lines 183–184:

> Every state change is a commit. `git log` is the complete audit trail. Commit trailers provide structured metadata for automated queries.

The recipe preserves this invariant by the simple expedient of *never calling `gh stack`*. The `gh stack` extension's rebase-and-force-push model never runs. There is no point at which the recipe force-pushes anything. There is no point at which the recipe rebases a LOOM branch.

### 5.2 The "audit-trail incompatibility" sharp edge is moot

Issue #74's sharp edge #4 reads:

> **Merge vs rebase.** LOOM does `--no-ff` merges for audit trails. `gh-stack` uses rebase + force-push. These are semantically incompatible for audit-log purposes.

This proposal's answer is blunt: the incompatibility only exists if someone invokes `gh stack`'s own rebase commands. The recipe refuses to do so, so the incompatibility never arises. This is not clever or controversial; it is a direct consequence of the angle.

The insight the writer must make explicit, because it is the load-bearing conceptual claim of §5: **stacked PRs and stacked branches managed by the gh-stack extension are not the same thing.** GitHub has supported PRs with arbitrary base branches since 2016. A PR with `base: loom/ratchet-feat-auth` is a stacked PR today, for any sensible definition of "stacked PR" — the reviewer sees only the diff between `loom/moss-feat-api` and its base `loom/ratchet-feat-auth`, which is the entire point of stacking for code review. That topology is achieved with a single `gh pr create --base`, which is exactly what `pr-create.ts` already does.

This proposal adopts the **PR topology** of stacking without adopting the **local-branch rebase workflow** of `gh-stack`. It picks the half of stacking that is already free in LOOM's existing toolchain and walks away from the half that would cost us the audit trail.

### 5.3 The honest tradeoff

If a lower layer is amended after its PR is created, the higher layers' PRs will show a confusing combined diff until they are either rebased (which this recipe refuses to do) or retargeted around the amendment (which this recipe does not provide for).

The recipe's answer is: **do not amend integrated layers.** This is already the LOOM norm, because `integrate()` is the point of no return for a worker's branch — once `integrate()` has been called, the branch is considered terminal and further commits to it are limited to orchestrator `chore(loom):` hotfixes (per `protocol.md` §2, "Orchestrator post-terminal commits"). The recipe's rule is the same rule LOOM already enforces, phrased slightly more strictly: *also* do not amend layers whose worker has reached `Task-Status: COMPLETED`, even if `integrate()` has not yet run.

This is a real restriction and the proposal acknowledges it as a cost. In exchange, the recipe preserves the byte-for-byte audit trail. The tradeoff is worth it because:

- The use case it forbids (amending a completed worker's branch after PR creation) is already rare in LOOM flows.
- The alternative (allowing amendment and rebasing higher layers) would require either invoking `gh stack rebase --upstack`, which breaks the audit trail, or re-implementing that logic ourselves, which is not zero code.
- If the restriction turns out to bite, the fix is to amend the recipe — not to amend the tools. We pay the cost later if and only if it is proven necessary.

### 5.4 Drift during long review cycles

If a reviewer sits on the stack for days while `main` advances, the bottom PR's merge will produce merge commits that higher layers have not seen. Because LOOM uses `--no-ff` and does not rebase, this manifests as a merge conflict at the time of the next `integrate()`, resolved exactly like any other conflict per `protocol.md` §5.2 (category `conflict`, retryable after rebase). The recipe does not introduce a new failure mode; it inherits the existing one and the existing recovery path.

Concretely: if layer 0 merges three days after creation, `main` has advanced, and layer 1's branch is now behind `main` plus the layer-0 merge commit, the orchestrator detects a merge conflict at `integrate()` time, abandons the merge (leaving workspace untouched per §3.3), spawns a fresh worker to rebase the layer-1 branch onto the new `main`, and retries. Same recovery path as a non-stacked LOOM worker whose branch fell behind `main` during review. Zero new cases to handle.

---

## 6. Worker authority

### 6.1 Workers invoke nothing new

Workers never invoke `gh stack`, `gh pr create`, `gh pr edit`, or any git command beyond the ones their current worker template already permits. The worker template is not modified by this proposal at all.

Per `protocol.md` §6.1 ("Trust boundary"):

> | Boundary | Rule |
> |---|---|
> | Workspace write | Only the orchestrator writes to the workspace. Agents MUST NOT. |
> | Agent scope | An agent may modify only files matching its `Scope` trailer. |

Neither of these rules is relaxed. The recipe slots entirely into the orchestrator's existing write-authority envelope — every new call is made by the orchestrator, about branches the orchestrator already owns the lifecycle of, using tools the orchestrator already has exclusive role access to.

### 6.2 The orchestrator is the only caller

`pr-create` and `pr-retarget` both declare `roles: ['orchestrator']` in their tool definition:

- `pr-create.ts` line 27: `roles: ['orchestrator']`
- `pr-retarget.ts` line 25: `roles: ['orchestrator']`

This proposal does not touch those role annotations. It inherits the existing gating for free. A worker trying to call `pr-create` with a stacked `base:` would be rejected at the MCP layer today, regardless of whether stacking is in play. The recipe does not widen that role list, does not introduce a new "stack-coordinator" role, and does not carve out any exception for stack-specific tool calls.

### 6.3 The recipe's single new responsibility

The recipe adds exactly one new orchestrator responsibility, and it is a **read-only** one:

> Topologically sort the `Dependencies:` trailers of a cohort of ASSIGNED branches before calling `pr-create` on them.

That is the entire new thing. It is a pure function of commit metadata the orchestrator already reads. It introduces no new write path, no new authority boundary, no new failure mode distinct from "`pr-create` rejected my `base:` argument" — which already exists and is already handled (`pr-create.ts` lines 43–44: `return err('pr-create-failed', ...)`).

Because the sort is read-only, it cannot corrupt any state. Because it operates on data the orchestrator already reads, it cannot introduce new data-access patterns. Because it affects only which string is passed as `base:`, it cannot affect worker behavior.

### 6.4 Contrast with angles that relax worker authority

Other teams in this round will likely propose designs where workers run `gh stack submit` from their worktrees. Such proposals need:

- **New scope rules** for `.git/refs/stacks/*` metadata files (or wherever `gh-stack` puts its local state) — these files are not code under `Scope:` today.
- **New heartbeat semantics** during the rebase windows that `gh stack submit` triggers — rebases can take minutes, during which the worker has no commits to checkpoint, which collides with `protocol.md` §8.1 ("Agents MUST include a `Heartbeat` trailer and commit at least every 5 minutes while running").
- **Relaxation of the workspace-write rule** in `protocol.md` §6.1, since `gh stack submit` force-pushes branches, which is a write to the repo's remote state that is currently orchestrator-only.
- **New prompt-injection surface**, because `gh stack` shells out to `git rebase` which shells out to editor hooks — a boundary the orchestrator-only model currently keeps tightly controlled.

This proposal needs **none** of that. The worker side of the protocol simply never learns that stacking is happening.

### 6.5 LOOM invariants affected

For completeness, here is the list of LOOM invariants this proposal interacts with and whether each is preserved or relaxed:

| Invariant | Source | Status |
|---|---|---|
| Only orchestrator writes workspace | `protocol.md` §6.1 | preserved |
| Workers scoped by `Scope:` trailer | `protocol.md` §6.1 | preserved |
| No cross-agent worktree writes | `protocol.md` §6.1 | preserved |
| `--no-ff` merge for integration | `protocol.md` §3.3, §8.2 | preserved |
| Branch naming `loom/<agent>-<slug>` | `schemas.md` §2 | preserved |
| Dependencies form a DAG | `schemas.md` §3.3 + `dag-check` | preserved |
| Heartbeat every 5 minutes | `protocol.md` §8.1 | preserved |
| `pr-create`/`pr-retarget` role-gated to orchestrator | tool definitions | preserved |
| First commit is `ASSIGNED` | `schemas.md` §7.2 | preserved |
| One agent per branch | `schemas.md` §2 | preserved |
| Terminal states are terminal | `schemas.md` §7.3 | preserved |

Count of invariants relaxed: **zero**.

---

## 7. End-to-end example

This section traces the canonical 3-agent epic from the RFP — "add auth middleware → API endpoints → frontend UI" — from decomposition to merged stack. Every `assign`, `dispatch`, `pr-create`, `push`, and `pr-retarget` call is spelled out with its exact arguments. The orchestrator is `bitswell`. The workers are `ratchet`, `moss`, and `ratchet` again (agents are reusable across assignments, per `protocol.md` §1).

### 7.1 Epic shape

- **Agent 1**: `ratchet`, assignment `feat-auth` — adds auth middleware. Dependencies: `none`.
- **Agent 2**: `moss`, assignment `feat-api` — adds API endpoints. Dependencies: `ratchet/feat-auth`.
- **Agent 3**: `ratchet`, assignment `feat-frontend` — adds frontend UI. Dependencies: `moss/feat-api`.

This is a linear `Dependencies:` chain, which is the precondition the recipe's "When to use it" check requires. The orchestrator verifies linearity by walking the DAG once with `dag-check` (existing tool, unchanged).

Branches (created by `assign()` at step 7.2):

- `loom/ratchet-feat-auth`
- `loom/moss-feat-api`
- `loom/ratchet-feat-frontend`

### 7.2 Step 1 — orchestrator assigns all three in one pass

The orchestrator calls the existing `assign` tool three times, once per agent, producing three ASSIGNED commits. Each commit uses the schema from `schemas.md` §5.1 exactly.

Assignment 1 (`loom/ratchet-feat-auth`), the ASSIGNED commit body:

```
task(ratchet): add auth middleware

Add Express-style auth middleware with JWT validation and
user context propagation. Exposes a `requireAuth()` helper
for downstream route handlers.

Agent-Id: bitswell
Session-Id: 7e3f8b4a-1234-4abc-9def-0123456789ab
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: feat-auth
Scope: src/auth/**
Dependencies: none
Budget: 40000
```

Assignment 2 (`loom/moss-feat-api`):

```
task(moss): add API endpoints

Add REST endpoints for user, session, and resource management.
Each endpoint uses `requireAuth()` from the auth middleware.

Agent-Id: bitswell
Session-Id: 7e3f8b4a-1234-4abc-9def-0123456789ab
Task-Status: ASSIGNED
Assigned-To: moss
Assignment: feat-api
Scope: src/api/**
Dependencies: ratchet/feat-auth
Budget: 40000
```

Assignment 3 (`loom/ratchet-feat-frontend`):

```
task(ratchet): add frontend UI

Add a React-based frontend calling the new API endpoints.
Uses the API client exposed by the api module.

Agent-Id: bitswell
Session-Id: 7e3f8b4a-1234-4abc-9def-0123456789ab
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: feat-frontend
Scope: src/frontend/**
Dependencies: moss/feat-api
Budget: 60000
```

Every ASSIGNED commit uses trailers already documented in `schemas.md` §3.3. Zero new trailers appear. The `Dependencies:` chain is linear — `dag-check` confirms it — which is the precondition for the stacked-PRs recipe.

### 7.3 Step 2 — orchestrator dispatches only the unblocked layer

The orchestrator calls `dispatch { branch: "loom/ratchet-feat-auth" }`. The other two branches are blocked on their dependencies and remain in `ASSIGNED` state; `loom-dispatch` already enforces this, per `protocol.md` §4.1:

> `loom-dispatch` checks dependencies before spawning. Blocked assignments remain ASSIGNED until dependencies are met.

No new logic runs. The recipe does not change dispatch behavior in any way.

### 7.4 Step 3 — ratchet drives `feat-auth` to COMPLETED

`ratchet` reads the ASSIGNED commit, commits IMPLEMENTING, writes the auth middleware, heartbeats every 5 minutes, and reaches COMPLETED with the standard final commit:

```
feat(auth): add JWT-validating requireAuth middleware

Implements requireAuth() with JWT validation, user context
injection, and 401 on failure. Full unit coverage.

Agent-Id: ratchet
Session-Id: 9a2c5f1b-abcd-4ef0-8901-234567890abc
Task-Status: COMPLETED
Files-Changed: 4
Key-Finding: JWT validation uses shared secret from env
Heartbeat: 2026-04-14T14:32:01Z
```

The orchestrator picks up the COMPLETED state via `git log -1 --format='%(trailers:key=Task-Status,valueonly)' loom/ratchet-feat-auth` (existing query, `schemas.md` §6).

### 7.5 Step 4 — orchestrator pushes the branch and creates the bottom-of-stack PR

First, the existing `push` tool is called to ensure the remote has the branch:

```
push { branch: "loom/ratchet-feat-auth" }
```

Then the orchestrator calls `pr-create`:

```
pr-create {
  head:  "loom/ratchet-feat-auth",
  base:  "main",
  title: "feat(auth): add JWT-validating requireAuth middleware",
  body:  "<rendered from ASSIGNED commit body + Key-Finding>"
}
```

This is the **bottom of the stack**. Its `base` is `main`. This is exactly the call the orchestrator would make today in a non-stacked LOOM run; the recipe does not alter this call at all. No special flag, no new argument, no side-band stack-id. The returned `{url, number}` is logged to the orchestrator's audit log (`pr-create.ts` line 53).

Let us say the returned PR number is **#101**.

Critically: the orchestrator does **not** call `integrate()` on `loom/ratchet-feat-auth` yet. This is what makes the stack real. The branch stays open as a reviewable PR; the next layer stacks on top of it; integration happens only after the PR merges via GitHub's merge button (or, later, `pr-merge`).

### 7.6 Step 5 — orchestrator dispatches `moss-feat-api`

Now that `loom/ratchet-feat-auth` has `Task-Status: COMPLETED`, `moss-feat-api`'s dependency is met. The orchestrator calls:

```
dispatch { branch: "loom/moss-feat-api" }
```

`loom-dispatch` sees the dependency satisfied and spawns `moss`. No new dispatch logic; `protocol.md` §4.1 already handles this exactly.

`moss` reads the ASSIGNED commit and begins work. An important subtlety: `moss` commits to `loom/moss-feat-api`, which was created by `assign()` from `main`, *not* from `loom/ratchet-feat-auth`. This means `moss`'s worktree does not contain `ratchet`'s auth code.

This looks like a problem. It is not, for three reasons:

1. **LOOM already has this property today** for any inter-agent dependency. `moss` would have had to read `ratchet`'s code across branches in any case. The recipe does not create the problem; it inherits it.
2. **The `integrate()` step at the end of `moss`'s lifecycle is responsible for merging `ratchet`'s work into `main` before `moss`'s branch is merged.** Because the recipe delays that integration until after review, the merged `main` is still reachable from both branches in the reflog — just not yet merged.
3. **The PR UI does the right thing.** When the orchestrator calls `pr-create` in step 5.7 below with `base: loom/ratchet-feat-auth`, GitHub renders only the diff between `loom/moss-feat-api` and `loom/ratchet-feat-auth` — which is exactly the layer-level diff the reviewer wants, regardless of whether `main` contains `ratchet`'s code yet.

If the worker needs to actually *execute* `ratchet`'s auth code (e.g., run integration tests that call `requireAuth()`), the worker's setup phase reads the dependency's branch and checks it in locally. This is already how LOOM handles cross-branch dependencies today.

### 7.7 Step 6 — moss completes, orchestrator creates the middle-of-stack PR

`moss` reaches COMPLETED with a standard final commit. The orchestrator runs:

```
push { branch: "loom/moss-feat-api" }
```

Then:

```
pr-create {
  head:  "loom/moss-feat-api",
  base:  "loom/ratchet-feat-auth",
  title: "feat(api): add REST endpoints using requireAuth",
  body:  "<rendered from ASSIGNED commit body + Key-Finding>"
}
```

**This is the load-bearing call in the entire proposal.** `base:` is literally the previous layer's LOOM branch name — `loom/ratchet-feat-auth`, the exact same string that `pr-create` in step 7.5 used as its `head`. `pr-create.ts` line 35 forwards this `base` verbatim to `gh pr create --base`. GitHub accepts it (because branches with non-trunk bases have been supported since 2016) and renders PR #102 as a diff between `loom/moss-feat-api` and `loom/ratchet-feat-auth`.

The reviewer opening PR #102 on GitHub sees only the API endpoints code — no auth middleware, no scaffolding, no unrelated churn. This is the per-layer review UX that is the whole point of stacking, achieved by calling a tool that already exists with an argument it already accepts.

Let us say the returned PR number is **#102**.

### 7.8 Step 7 — frontend layer

Same pattern again. The orchestrator dispatches:

```
dispatch { branch: "loom/ratchet-feat-frontend" }
```

`ratchet` (a new session, same agent) drives `feat-frontend` to COMPLETED. The orchestrator pushes and creates:

```
pr-create {
  head:  "loom/ratchet-feat-frontend",
  base:  "loom/moss-feat-api",
  title: "feat(frontend): add React UI for new API",
  body:  "<rendered from ASSIGNED commit body + Key-Finding>"
}
```

**Top of the stack**: `base` is the middle layer's branch. PR **#103** is created, showing only the frontend diff against the API layer.

At this point, three PRs exist on GitHub:

- PR #101: `loom/ratchet-feat-auth` → `main`
- PR #102: `loom/moss-feat-api` → `loom/ratchet-feat-auth`
- PR #103: `loom/ratchet-feat-frontend` → `loom/moss-feat-api`

Each PR shows a single layer's diff. GitHub's native UI renders them as a stack without any additional metadata. No `gh stack init` was ever called. No force-push ever happened. No rebase ever happened.

### 7.9 Step 8 — reviewer approves, orchestrator merges bottom-up

The reviewer approves all three PRs. The orchestrator merges bottom-up, because a stack must merge bottom-up:

**Merge #1**: PR #101 merges to `main`. The merge is a `--no-ff` merge exactly as today, producing a standard LOOM audit-trail commit. GitHub deletes the `loom/ratchet-feat-auth` branch from the remote on merge (assuming the repo has "automatically delete head branches" enabled; if not, a manual delete is fine too).

The moment `loom/ratchet-feat-auth` disappears from the remote, **PR #102's base is stranded**. GitHub handles this gracefully — it does not close the PR — but the PR now shows a confusing diff (`loom/moss-feat-api` against a nonexistent branch). The orchestrator runs the fixup:

```
pr-retarget { number: 102, base: "main" }
```

This is the exact case the existing `pr-retarget` tool was built for. Its handler at `pr-retarget.ts` lines 30–34 is literally `gh pr edit 102 --base main`. No new code path is exercised. GitHub re-renders PR #102 as a diff against `main`, which now contains `ratchet`'s auth code (merged in the previous step), so the diff shows exactly the API-layer changes — identical to what the reviewer saw before, minus the now-integrated dependency.

**Merge #2**: PR #102 merges to `main`. Another standard `--no-ff` merge. `loom/moss-feat-api` disappears. PR #103's base is stranded. The orchestrator runs:

```
pr-retarget { number: 103, base: "main" }
```

Same story: one `gh pr edit` call, existing tool, no new code path. PR #103 is now a diff against `main` (which contains both auth and API code), showing exactly the frontend-layer changes.

**Merge #3**: PR #103 merges to `main`. The stack is fully integrated. Three `--no-ff` merge commits appear on `main` in order. The audit trail is byte-identical to the audit trail of a non-stacked LOOM run that integrated the same three branches in the same order.

### 7.10 Tool-call surface area, summed

The complete set of new tool calls this recipe introduces — *new* meaning "calls the orchestrator would not already be making in a non-stacked LOOM run" — is:

- 3 × `pr-create` with non-`main` or non-trivial `base` (wait — actually, only 2 of the 3 have non-`main` `base`; PR #101 has `base: main` which is identical to the non-stacked flow)
- 2 × `pr-retarget` fixups after each non-top merge

Total: **2 pr-create calls with non-trivial base**, **2 pr-retarget calls**. Four orchestrator-side tool calls, against tools that already exist, all of which are already role-gated to the orchestrator, none of which exercise any new code path in `loom-tools`.

### 7.11 What did not happen

For clarity, here is what this example deliberately did **not** do:

- No `gh stack init`, `gh stack add`, `gh stack submit`, `gh stack sync`, or `gh stack view`.
- No `git rebase` on any LOOM branch.
- No force-push to any LOOM branch.
- No new trailer on any ASSIGNED commit.
- No new required field in `schemas.md`.
- No new worker role or new worker-template section.
- No relaxation of `protocol.md` §6.1.
- No release of `loom-tools`.
- No new commit on the worker side that was not already required by today's protocol.
- No new prompt injection surface.

Everything the reviewer saw, GitHub delivered natively from the PR base chain. Everything the orchestrator called, it already had. Everything the worker did, the worker template already told it to do.

---

## 8. Risks and rejected alternatives

### 8.1 Risks

**Risk 1 — Linear-only stacks.** The recipe only works when `Dependencies:` forms a single linear chain. Diamond-shaped DAGs (two independent branches whose work both feeds into a third) and fan-in DAGs cannot be rendered as a stack. This limitation is named honestly. The recipe's "When to use it" check returns false for non-linear DAGs, and the orchestrator falls back to the current non-stacked flow (each branch becomes an independent PR against `main`).

This is the same limitation `gh-stack` itself documents at `/home/willem/.agents/skills/gh-stack/SKILL.md` line 789:

> **Stacks are strictly linear.** Branching stacks (multiple children on a single parent) are not supported. Each branch has exactly one parent and at most one child.

Accepting the same limitation as the tool we are trying to interoperate with is not ceding ground. A heavier proposal that promised to render diamond DAGs as stacks would be overreaching: even if LOOM built the machinery, GitHub's native stacked-PR UX would not render it meaningfully, because GitHub's model is also "one base per PR." The correct thing to do with a diamond DAG is not to stack it — it is to ship the two independent legs as independent PRs and stack only the merged tip, which this recipe already does.

**Risk 2 — Base-branch deletion on merge.** When a lower PR merges, its branch is deleted from the remote and higher PRs' bases go stale until the orchestrator retargets them. The window between merge and retarget is a period during which the higher PR shows a confusing diff.

The mitigation is the `pr-retarget` call in §7.9 step 8. The residual risk is that the orchestrator forgets to run it. The consequence of forgetting is cosmetic — a confused-looking PR — not catastrophic; no data is lost, no branch is corrupted. The orchestrator can recover at any later point by running the missed `pr-retarget`, or by closing and re-opening the higher PR.

A possible amplification: if GitHub's "automatically delete head branches" setting is off and the orchestrator also does not manually delete the lower branch, the higher PR's base remains valid but points at a dead branch that will accumulate drift from `main` until someone notices. The mitigation is the same: call `pr-retarget` as part of the merge recipe, regardless of whether the branch was deleted.

**Risk 3 — Drift during long review cycles.** If a reviewer sits on the stack for days while `main` advances, the bottom PR's merge will introduce merge commits that higher layers have not seen. Because LOOM uses `--no-ff` and never rebases, this manifests as a merge conflict at the next `integrate()` — which is the exact category of failure `protocol.md` §5.2 already calls `conflict`, with a defined retryable recovery path. The recipe does not introduce a new failure mode here; it inherits the existing one.

Concretely, if PR #101 merges three days after creation and `main` has advanced in the meantime, PR #102's `base` is still `main` (after the `pr-retarget` in step 7.9), and `main` now contains both the unrelated advances and the merged auth code. Any textual collision between `main`'s advances and `moss`'s code manifests as a merge conflict when PR #102 is merged. The reviewer (or orchestrator) resolves it the same way any other conflict is resolved in LOOM today: spawn a fresh worker with a narrow scope to rebase `loom/moss-feat-api` onto the new `main` head, then re-try the merge.

**Risk 4 — Reviewers expecting `gh-stack` UX.** A human reviewer familiar with `gh stack view --json` (or, worse, the interactive TUI that `gh stack view` without `--json` opens) will find nothing to view, because no `gh stack` state exists locally. They may assume the repository is not actually set up for stacking. The mitigation is documentation: the `stacked-prs.md` recipe page includes a short "for reviewers" section explaining that GitHub's native stacked-PR UI renders correctly from the PR base chain alone, no `gh stack` needed. The secondary mitigation is that most LOOM reviews are done through the GitHub web UI, not through `gh stack`.

**Risk 5 — PR body drift.** `pr-create`'s `body:` argument is today populated from the ASSIGNED commit body. In a stacked flow, each layer's PR body currently makes no reference to the layers below it. A reviewer reading PR #103 in isolation has no cue that it sits on top of two unmerged PRs. The recipe includes a suggested convention: prepend a "Stack context" section to the PR body naming the immediate `base:` and the stack's full chain. This is a convention *within* the recipe, not a tool change — the orchestrator just passes a slightly richer `body:` string to `pr-create`, which already accepts an optional string.

**Risk 6 — No merge-queue semantics.** GitHub's merge queue does not (as of this writing) handle stacked PRs well — if enabled, the queue may try to merge PRs out of stack order. The mitigation is that the recipe explicitly disables merge queue for stacked epics (or, equivalently, the orchestrator serialises merges bottom-up manually, which is what step 7.9 already does). This is named as a limitation, not papered over.

### 8.2 Rejected alternatives

Three alternatives were considered and rejected during the outline phase. Each is documented with its specific reason.

**Rejected alternative 1 — Add a `Stack-Position:` trailer to ASSIGNED commits.**

The shape: extend `schemas.md` §3.3 to add an optional `Stack-Position: <n>/<total>` trailer on ASSIGNED commits, so the recipe can read stack position directly instead of computing it from the `Dependencies:` topological sort.

Why it was tempting: it makes the stack structure explicit in the commit message, which is easier to grep for than the sort-and-walk approach.

Why it was rejected:

1. The information is already encoded in `Dependencies:`. Adding `Stack-Position:` duplicates it, and duplicated metadata drifts — the two sources can get out of sync if the orchestrator updates one and not the other.
2. A new trailer in `schemas.md` §3.3 forces *every* team using LOOM to either provide it or have the orchestrator compute a default. Even teams not using stacked PRs pay the cognitive cost of the new field.
3. The topological sort is O(n) on the number of agents in the epic, which is typically three to ten. The "efficiency" argument for precomputing stack position is not real at this scale.
4. Any information the orchestrator needs can be derived from `Dependencies:` at recipe-execution time, at zero cost. Adding a trailer to avoid the derivation is premature optimisation of the wrong kind.

This rejection is load-bearing for the "zero schema changes" property of §3.3.

**Rejected alternative 2 — Add a new `stack-create` MCP tool that wraps the recipe.**

The shape: expose the recipe as a first-class tool in `loom-tools`, something like:

```ts
stack-create {
  branches: string[],  // in stack order, bottom to top
  titles: string[],    // one per branch
}
```

which internally performs the N `pr-create` calls.

Why it was tempting: a single tool call is ergonomically nicer than a recipe the orchestrator has to remember.

Why it was rejected:

1. Moving the recipe from documentation into executable code locks in the recipe's *shape* before we have evidence it is the right shape. The whole point of the convention-only angle is to ship the cheapest thing that could possibly work, learn from it, and then promote it if it turns out to be right.
2. A new tool forces a `loom-tools` release cycle for every tweak to the recipe. If we discover after the first real stacked epic that we want to retarget PRs differently, that is a documentation edit in the recipe-page model and a new MCP release in the tool model.
3. The tool would have to be role-gated to orchestrator, which means the permission story is identical — there is no benefit to being a tool in terms of authority.
4. The tool's input schema would need to encode stack-ordering information that is already in `Dependencies:` — we would either be asking the caller to pre-sort (duplicating the recipe's logic), or we would have to look up `Dependencies:` inside the tool (adding a new read-side dependency on git state to a previously pure tool).
5. If the recipe proves correct after real use, promoting it to a tool is a one-file PR later — we do not lose the option to make this change in a follow-up round.

In short: this alternative is exactly the kind of "ship it as code" commitment the convention-only angle is designed to *defer*. Deferring is not refusing — it is choosing the cheap experiment first.

**Rejected alternative 3 — Invoke `gh stack init --adopt` and `gh stack submit --auto`.**

The shape: at the point in the flow where this proposal calls `pr-create`, instead run `gh stack init --adopt` on the already-existing LOOM branches, then `gh stack submit --auto` to push them all as a stack.

Why it was tempting: it would give LOOM the full `gh stack` feature set for free — `gh stack view --json` for introspection, `gh stack sync` for drift handling, `gh stack rebase --upstack` for amendments.

Why it was rejected:

1. **Audit-trail incompatibility.** `gh stack submit --auto` performs rebases and force-pushes (see SKILL.md sections on `gh stack submit` and `gh stack rebase`). Rebasing a LOOM branch violates `protocol.md` §8.2: "Every state change is a commit. `git log` is the complete audit trail." A rebase rewrites state changes that have already been committed, which means the audit trail is no longer complete — prior commit SHAs are now unreachable except via reflog.
2. **Force-push violates the orchestrator-only workspace-write rule.** `protocol.md` §6.1 forbids workers from writing to the workspace; if we extend this to mean "force-pushing a branch is a write to the shared remote state," then letting `gh stack` force-push on the orchestrator's behalf at least requires auditing what exactly is being force-pushed when, and that audit is not cheap.
3. **New scope rules for stack metadata.** `gh stack` stores stack-level metadata in `.git/refs/stacks/*` and similar locations. These files are not under any agent's `Scope:` today, and adding them is not trivial — `Scope:` is a per-agent construct, not a shared-repo construct.
4. **New interactive-prompt risk.** Several `gh stack` subcommands are interactive by default and must be invoked with flags to avoid hanging (see SKILL.md "Agent rules"). Wrapping them in a LOOM tool without the `--auto`, `--json`, and `--remote` flags correctly set is a latent hang bug.
5. **No benefit that `pr-create --base` does not already deliver.** The per-layer review UX that is the whole point of stacking is delivered by GitHub's native stacked-PR rendering as soon as the PR chain exists. `gh stack view` is nice for humans driving stacks locally, but LOOM's orchestrator does not need it — it already knows the stack structure from `Dependencies:`.

This alternative re-opens every sharp edge the recipe was designed to sidestep, in exchange for no benefit that `pr-create --base` does not already deliver. Rejected decisively.

### 8.3 A fourth alternative worth naming — "do nothing"

There is a meta-alternative: reject the whole RFP and leave LOOM without stacked-PR support. This proposal does not argue for "do nothing" — the reviewer UX benefit of per-layer diffs is real and the RFP explicitly assumes the answer is yes. But for the sake of honest bookkeeping: "do nothing" is strictly dominated by *this* proposal, which ships with zero code changes. Any argument for "do nothing" on cost grounds applies equally to every other proposal in this round, and applies more weakly to this one than to any of the alternatives.

---

## 9. Summary

- **Angle**: convention-only, zero code changes.
- **Delta**: one new documentation page (`stacked-prs.md`), one single-line addition to the plugin `SKILL.md` references list.
- **Code changed**: zero lines.
- **New tools**: none.
- **New trailers**: none.
- **New worker behavior**: none.
- **LOOM invariants relaxed**: none.
- **Audit trail property**: byte-identical to non-stacked LOOM.
- **Worker authority**: unchanged; only the orchestrator invokes `pr-create` and `pr-retarget`, exactly as today.
- **Tool-call surface area on the critical path for a 3-agent stacked epic**: three `pr-create` calls and two `pr-retarget` calls — all against tools that already exist.
- **Constraint accepted**: linear stacks only. Diamonds fall back to the non-stacked LOOM flow.
- **Ship window**: the afternoon of approval.

The load-bearing claim of the proposal: **stacked PRs and stacked branches managed by the `gh-stack` extension are not the same thing.** GitHub has supported PRs with arbitrary base branches for years; `pr-create --base loom/ratchet-feat-auth` produces a stacked PR today, using only `gh pr create` under the hood. We are adopting the PR topology without adopting the rebase workflow, and in doing so we get the reviewer UX benefit of stacking without paying the audit-trail cost of `gh stack`.

If a later round builds heavier machinery on top of this baseline, the recipe survives as the documented fallback. If it does not, the recipe is the permanent solution. Either way, shipping this costs the project nothing it cannot un-ship.

---

## Appendix A — Key references

| Reference | Location | What it proves |
|---|---|---|
| Issue #74 sharp edge #1 | bitswell/bitswell#74 | "A minimal-viable stacking path needs no tool changes at all." The invitation for a zero-code proposal. |
| `pr-create.ts` schema | `repos/bitswell/loom-tools/src/tools/pr-create.ts` lines 6–11 | `base: z.string()` with no restriction on its value. |
| `pr-create.ts` handler | `repos/bitswell/loom-tools/src/tools/pr-create.ts` lines 32–42 | Forwards `--base` verbatim to `gh pr create`. |
| `pr-create.ts` role gating | `repos/bitswell/loom-tools/src/tools/pr-create.ts` line 27 | `roles: ['orchestrator']`. |
| `pr-retarget.ts` schema | `repos/bitswell/loom-tools/src/tools/pr-retarget.ts` lines 6–9 | `{number, base}` input. |
| `pr-retarget.ts` handler | `repos/bitswell/loom-tools/src/tools/pr-retarget.ts` lines 30–34 | `gh pr edit <number> --base <base>`. |
| `pr-retarget.ts` role gating | `repos/bitswell/loom-tools/src/tools/pr-retarget.ts` line 25 | `roles: ['orchestrator']`. |
| `schemas.md` §3.3 `Dependencies:` | `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` lines 72–82 | Existing trailer the recipe reads; no changes. |
| `schemas.md` §2 branch naming | same file lines 34–52 | `loom/<agent>-<slug>`; the recipe uses this verbatim. |
| `protocol.md` §3.3 `integrate()` | `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` lines 86–98 | `--no-ff` merge is the integration primitive; preserved. |
| `protocol.md` §8.2 audit trail | same file lines 183–184 | "Every state change is a commit. `git log` is the complete audit trail." The invariant the recipe preserves. |
| `protocol.md` §4.1 `loom-dispatch` | same file lines 104–114 | Dependency gating is already automatic; the recipe inherits it. |
| `protocol.md` §6.1 trust boundary | same file lines 157–165 | The table the recipe does not touch. |
| `gh-stack` SKILL "Known limitations" #1 | `/home/willem/.agents/skills/gh-stack/SKILL.md` line 789 | "Stacks are strictly linear." The recipe accepts the same limitation. |
| Plugin skill references dir | `plugins/loom/skills/loom/references/` in loom-plugin repo | The four existing peer pages; the new `stacked-prs.md` sits here. |
| `examples.md` | same directory | Precedent for narrative recipes composed from existing tools. |

---

## Appendix B — The `stacked-prs.md` recipe page, in outline

For the reader who wants to know exactly what the one new file in this proposal would contain, here is its outline. This is not the file itself — that ships with the implementation round — but it is concrete enough for a reviewer to judge what this proposal commits to.

```
# Stacked Pull Requests (LOOM Recipe)

A recipe for shipping a linear chain of dependent LOOM assignments as
stacked pull requests, using only existing tools.

## When to use this recipe

- You have 2+ LOOM assignments whose `Dependencies:` form a linear chain.
- You want each layer reviewable in isolation (per-layer diff).
- You want the LOOM audit trail preserved byte-identically.

If the DAG forks, use the standard LOOM flow instead: each branch becomes
an independent PR against `main`.

## How it works

1. Topologically sort the cohort of ASSIGNED branches by `Dependencies:`.
   Call this the stack order. The first branch is the bottom; the last is
   the top.

2. Dispatch and let each layer complete before starting the next, exactly
   as today.

3. After each layer reaches `Task-Status: COMPLETED`, the orchestrator
   calls `push` followed by `pr-create`:

     pr-create {
       head:  "loom/<this-agent>-<this-slug>",
       base:  <previous layer's branch>  // or "main" for layer 0
       title: <derived from assignment>,
       body:  <derived from ASSIGNED commit>
     }

4. When a lower PR merges, run `pr-retarget` on the immediately-higher PR:

     pr-retarget {
       number: <higher PR number>,
       base:   "main"                     // or next-lowest still-open branch
     }

5. Do NOT call `gh stack` at all. Not `init`, not `add`, not `submit`,
   not `sync`, not `rebase`, not `view`. The extension is about local-branch
   topology management and is redundant here; its rebase model is also
   incompatible with LOOM's `--no-ff` audit trail.

## Worked example

[mirrors §7 of the proposal]

## For reviewers

GitHub renders stacked PRs natively from the PR base chain. You do not need
`gh stack view`. Each PR's diff is the diff of that layer alone. Merge
bottom-up.

## Limitations

- Linear stacks only.
- Do not amend layers that have reached `Task-Status: COMPLETED`.
- Base-branch deletion on merge requires a `pr-retarget` fixup (built into
  this recipe).
```

That is the full shape of the new file. It sits alongside `examples.md`, `protocol.md`, `schemas.md`, and `worker-template.md` in the plugin skill references directory. It is the entire implementation of this proposal.

---

*End of team-2 proposal. One angle, zero code changes, one new page.*
