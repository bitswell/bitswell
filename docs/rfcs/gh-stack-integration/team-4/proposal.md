# Team 4 — Proposal: Worker-side stacking with delegated authority

**Epic:** [#74 — gh-stack integration](https://github.com/bitswell/bitswell/issues/74)
**Team angle key:** *worker-authority stacking* (a.k.a. *delegated-stack stacking*, *worker-driven stacking*)
**Explicit non-angle:** this proposal is **not** post-hoc projection. Post-hoc projection is team 1's reassigned angle and any sentence in this document implying workers only drive stacking after integration is a drafting bug.

---

## 1. Angle statement

**Worker-side stacking with delegated authority: for stack-epic assignments, LOOM explicitly delegates `gh stack` authority to workers, who `init`, `add`, `rebase`, and `submit` from inside their own worktrees, and the trust model shifts from "orchestrator owns all GitHub writes" to "orchestrator owns the audit contract, workers own the stack mechanics."**

That sentence is the contract. To make it impossible to misread, this proposal repeats the claim three times in different registers:

- **In one sentence, mechanically:** When an assignment carries `Stack-Epic: true`, the worker runs `gh stack init`, `gh stack add`, `gh stack rebase`, `gh stack submit`, `gh stack sync`, `gh stack push`, `gh stack view --json`, and every `gh stack` navigation command directly from its worktree. The orchestrator does not run any `gh stack` command for that epic.
- **In one sentence, architecturally:** The trust boundary between orchestrator and worker is redrawn so that the orchestrator retains exclusive authority over the *contract* (assignments, scope, integration, audit) while workers gain exclusive authority over the *mechanics* of stacking within their own epic's branch namespace.
- **In one sentence, reductively:** *Proximity of data beats protocol purity.* `gh stack` was built on the assumption that the person making the commits also drives the stack. This proposal aligns LOOM with that assumption instead of fighting it.

The collision-detection key is **worker-authority stacking**. This angle is disjoint from team 1's post-hoc projection (stack is assembled after integration), from any proposal that keeps workers on the orchestrator side of `gh stack`, and from any proposal that tries to satisfy the epic by adding an MCP wrapper tool with `roles: ['orchestrator']`. If the winning proposal does not let workers run `gh stack` commands, it is not this proposal.

**The bet, stated as a tradeoff the reviewer must accept or reject:** Worker-driven conflict recovery and mid-stack edits are worth a deliberate, documented relaxation of LOOM's workspace-write and `--no-ff` audit invariants. This proposal does not try to soften the tradeoff. If the reviewer believes that no GitHub write should ever leave the orchestrator, this proposal is the wrong bet and team 1 is the right one.

---

## 2. Thesis

Every other plausible integration path leaves the worker out of the `gh stack` call path and pays a price for it:

- **Post-hoc projection** (team 1) produces a single pile of commits from the worker and projects a stack onto them after the fact. That works for clean linear work but cannot react to *live* conflicts: a worker that needs to revise layer 1 while working on layer 3 has no stack tools to navigate down, edit, and rebase upstack. The projection only exists in the orchestrator's head until integration time, which is exactly when conflict recovery has already been lost.
- **Orchestrator-only MCP wrapper** (the obvious path, rejected alternative B below) forces every stack operation to round-trip through the orchestrator. That serializes conflict recovery on a process with no working tree and no `rerere` cache, and it means orchestrator context compaction silently destroys mid-rebase state.
- **Convention-only recipes** that tell workers "don't stack, just commit, and the orchestrator will handle stacks later" are just post-hoc projection with worse ergonomics. They also require the orchestrator to pretend to be the branch author at submit time, which confuses `gh-stack`'s audit output.
- **Pre-create every stack branch upfront** (rejected alternative C) tries to make stacks declarative so the orchestrator can stamp branches and workers can just commit to them. That works until the first mid-stack edit, at which point it collapses into one of the above.

All four fight the grain of `gh-stack`. The skill at `/home/willem/.agents/skills/gh-stack/SKILL.md` is explicit on this point: agent rule 9 reads *"Navigate down the stack when you need to change a lower layer."* That instruction is written to the agent-that-has-the-worktree, not to a remote coordinator. `gh stack` exit code 8 (*"Stack is locked"*) is produced by a worktree-level lock file that cannot be safely shared across processes. The `git rerere` cache that powers smooth rebases lives inside `.git/rr-cache` of the worktree doing the rebase — not the orchestrator's worktree, and not any shared location. Every one of these facts says the same thing: **the party that makes the commits must also drive the stack.**

This proposal accepts that message and reorganises LOOM's trust model around it. For stack-epic assignments, the worker is both the branch author *and* the stack driver, full stop. The orchestrator retreats from being a gatekeeper of mechanism and becomes a verifier of contract. It still owns ASSIGNED commits, scope verification, integration, and the audit reconstruction — but it no longer serializes on every force-push, every `rerere` lookup, every mid-rebase decision.

The cost is real and named in the angle statement itself. LOOM's `protocol.md §6.1` "Workspace write" invariant must be deliberately relaxed for stack-epic workers. LOOM's `protocol.md §3.3` `--no-ff` integration step must be replaced with cascading `--ff-only` for stack-epic branches. LOOM's worker template `§9` scope enforcement must be relaxed to a union-scope model. And a new role tier — `worker-stack-driver` — must be introduced into `repos/bitswell/loom-tools/src/types/tool.ts` so that the tool schema can express "this tool is callable by a worker only when the assignment carries `Stack-Epic: true`."

This proposal is the only one of the five that names that cost in its angle sentence. Every other team hides the cost behind a projection trick or an MCP wrapper. The reviewer should read this proposal as the one that tells the truth about what a real `gh-stack` integration costs LOOM, and then asks: given the cost, is worker-driven conflict recovery worth paying? The thesis is that it is, because conflict recovery and mid-stack edits are exactly the operations that make stacked diffs valuable in the first place. A stack that can only be built from green-path clean commits is not a stack — it is a linear diff with extra ceremony. A stack that workers can edit, rebase, and recover under pressure is the thing that justifies the tooling.

---

## 3. What changes

This section enumerates every component that changes, every file path that is touched, and every component that explicitly does not change. Where an existing file is the template for a new one, that relationship is called out.

### 3.1 `loom-tools` — new tool

**New file:** `repos/bitswell/loom-tools/src/tools/stack-worker-init.ts`

**Role:** `worker-stack-driver` (newly introduced — see §3.2).

**Template:** modelled on `repos/bitswell/loom-tools/src/tools/pr-create.ts`, with the following differences:

1. The `roles` field is `['worker-stack-driver']`, not `['orchestrator']`.
2. The tool does not wrap `gh pr create`. It wraps the `gh stack` subcommand tree.
3. The input schema takes an `op` discriminator (`'init' | 'add' | 'rebase' | 'rebase-continue' | 'rebase-abort' | 'submit' | 'sync' | 'push' | 'view' | 'checkout' | 'up' | 'down' | 'top' | 'bottom' | 'unstack'`) plus per-op arguments. Each op invocation is non-interactive (SKILL §"Agent rules") and auto-stamps a `Stack-Op:` trailer on the commit that follows.
4. The handler refuses to run if `ctx.assignment` does not carry `Stack-Epic: true`. This is the single defense that keeps non-stack workers from calling into the `gh stack` subcommand tree.
5. The handler refuses `submit` unless `--auto --draft` is set. Non-draft submission is reserved for integration-time GitHub state changes, which are still orchestrator-only.

**Registration:** added to `repos/bitswell/loom-tools/src/tools/index.ts` alongside existing exports. No other tools are renamed or removed.

**Unchanged tools:**

- `repos/bitswell/loom-tools/src/tools/pr-create.ts` — `roles: ['orchestrator']` stays. Non-stack PR creation is still orchestrator-only.
- `repos/bitswell/loom-tools/src/tools/pr-retarget.ts` — `roles: ['orchestrator']` stays. Stack bases are set by `gh stack submit`, not `pr-retarget`, so there is no overlap with the new tool.
- `repos/bitswell/loom-tools/src/tools/dag-check.ts` — no changes to the topological sort. For stack epics the sort runs over a single assignment whose `Stack-Position:` trailer declares the layer order, so the existing algorithm returns `[layer-1, layer-2, layer-3, ...]` unchanged.

### 3.2 Role enum expansion

**File:** `repos/bitswell/loom-tools/src/types/tool.ts`

**Current state:** `roles` is effectively binary — tools are either `['orchestrator']` (see `pr-create.ts` line 27) or unrestricted. The type definition that gates this is the `roles` field on the `ToolDefinition` interface.

**Change:** add a third role value, `worker-stack-driver`, as a strict superset of `worker` for stack-epic assignments only.

```ts
// repos/bitswell/loom-tools/src/types/tool.ts (amended)
export type Role = 'orchestrator' | 'worker' | 'worker-stack-driver';
export interface ToolDefinition<I, O> {
  name: string;
  description: string;
  inputSchema: z.ZodType<I>;
  outputSchema: z.ZodType<O>;
  roles: Role[];
}
```

**Enforcement:** the MCP dispatch layer already checks `roles` on every tool call and rejects unauthorised callers. The new role value is checked the same way. A worker running without `Stack-Epic: true` cannot successfully call `stack-worker-init` because the handler refuses and because the dispatch guard blocks the `worker-stack-driver` role from being asserted on non-stack-epic sessions.

### 3.3 Plugin / loom skill surface

**File:** new recipe — `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/stack-epic-recipe.md`

This recipe is loaded by the orchestrator during decomposition when an epic is identified as stack-mode. It describes:

1. How to identify a stack epic (the heuristic: at least two layers with linear dependencies and a shared reviewer).
2. How to emit a `Stack-Epic: true` ASSIGNED commit with `Stack-Position:` laid out.
3. How to run cascading `--ff-only` integration (§4).
4. How to reconstruct the audit trail post-integration via `git log --format='%(trailers:key=Stack-Op)'`.

**File:** amendment to `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/worker-template.md`

Add **Section 10 — "Stack-epic workers"** which overrides Section 9 ("Scope Enforcement") with the relaxed rules from §5 of this proposal. The amendment reads, in full:

> **10. Stack-epic workers**
>
> If your AGENT.json's ASSIGNED commit carries `Stack-Epic: true`, the following overrides apply to §4 (Do the Work) and §9 (Scope Enforcement):
>
> 10.1. You are authorised to run `gh stack init`, `gh stack add`, `gh stack rebase` (including `--continue` and `--abort`), `gh stack submit --auto --draft`, `gh stack sync`, `gh stack view --json`, `gh stack push`, `gh stack checkout`, `gh stack up/down/top/bottom`, and `gh stack unstack` from inside your worktree. All other `gh` commands remain forbidden — in particular, `gh pr create`, `gh pr merge`, and `gh pr edit` are orchestrator-only.
>
> 10.2. Your scope (§9) is enforced as the *union* of all layer scopes declared in `Stack-Position`. Every commit on every branch you create via `gh stack add` is checked against this union at integration time.
>
> 10.3. Every `gh stack` command you run must be followed by a commit with a `Stack-Op: <op>` trailer. If the command produces no file changes, use `git commit --allow-empty` to keep the audit trail complete.
>
> 10.4. If a rebase conflicts, follow the SKILL's "Handle rebase conflicts" procedure: parse stderr, edit files, `git add`, `gh stack rebase --continue`, then commit with `Stack-Op: rebase-continue`. Do not force-push outside your stack's branch namespace.
>
> 10.5. Final submission is `gh stack submit --auto --draft` followed by a COMPLETED commit whose body lists the draft PR numbers. The orchestrator, not you, promotes drafts and merges.
>
> 10.6. You are the sole worker for the stack. Do not spawn sub-workers. Do not run `gh stack` commands on any stack whose namespace does not match `loom/<your-agent-id>-<assignment-slug>/*`.

### 3.4 Schemas

**File:** `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md`

**Amendments:**

- **§3.3 assignment trailers** — add:
  - `Stack-Epic: true | false` — REQUIRED on stack-epic ASSIGNED commits, FORBIDDEN otherwise. Default value when omitted is `false`.
  - `Stack-Position: <layer-1>,<layer-2>,...` — REQUIRED when `Stack-Epic: true`. Lists layer slugs in bottom-to-top order. Example: `Stack-Position: 1:auth-middleware,2:api-endpoints,3:frontend`.
- **§3.2 state trailers** — add:
  - `Stack-Op: init | add | rebase | rebase-continue | rebase-abort | submit | sync | push | view | checkout | up | down | top | bottom | unstack | commit` — REQUIRED on every commit a stack-epic worker makes that is not an IMPLEMENTING regular content commit. (Regular content commits carry `Stack-Op: commit` to keep the audit trail regular.)
- **§4.1 required trailers per state** — extend the table for stack-epic branches:

| State | Required trailers (stack-epic delta) |
|-------|--------------------------------------|
| ASSIGNED | `Stack-Epic: true`, `Stack-Position: <...>` |
| IMPLEMENTING | `Stack-Op: <op>` on every commit |
| COMPLETED | `Stack-Op: submit`, `Files-Changed: <n>`, `Key-Finding: <...>` at least once |

Non-stack branches are unaffected.

### 3.5 Worker template

Already described in §3.3 above. To be explicit about the file: `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/worker-template.md` gains a new §10 and picks up one-line cross-references in §4 ("If stack-epic, see §10") and §9 ("See §10 for the stack-epic override").

### 3.6 Explicit no-change list

- `repos/bitswell/loom-tools/src/tools/pr-create.ts` — no changes.
- `repos/bitswell/loom-tools/src/tools/pr-retarget.ts` — no changes.
- `repos/bitswell/loom-tools/src/tools/dag-check.ts` — no changes.
- `repos/bitswell/loom-tools/src/tools/pr-merge.ts` — no changes.
- `protocol.md §2` (state machine) — no textual change; the one-agent-per-branch rule is documented as having an exception carved out by §10 of the worker template, which is a reference change only.
- `protocol.md §3.1` (assign) and `§3.2` (implement) — no changes to the state machine itself.
- Everything outside the paths enumerated in §3.1–§3.5.

---

## 4. Branch naming and scope

### 4.1 Branch naming convention

LOOM's current convention is `loom/<agent>-<slug>`, one branch per assignment. For stack epics this proposal extends the convention to a compound form:

```
loom/<lead-agent>-<epic-slug>/<layer-slug>
```

- `<lead-agent>` is the single worker that owns the whole stack (see §4.2).
- `<epic-slug>` is the assignment slug.
- `<layer-slug>` is one of the layer names declared in `Stack-Position`.

Concretely, for `ratchet` working on an auth epic with layers `auth-middleware`, `api-endpoints`, `frontend`:

```
loom/ratchet-auth-stack/auth-middleware   ← bottom
loom/ratchet-auth-stack/api-endpoints
loom/ratchet-auth-stack/frontend           ← top
```

`gh-stack`'s prefix mechanism lines up with this exactly. The SKILL §"Branch naming" and §"Prefix handling" rules say: *"Only pass the suffix when a prefix is set."* So the worker runs `gh stack init -p loom/ratchet-auth-stack auth-middleware`, which creates `loom/ratchet-auth-stack/auth-middleware`. Subsequent `gh stack add api-endpoints` creates `loom/ratchet-auth-stack/api-endpoints`. The `loom/` namespace is preserved, the `<lead-agent>-<epic-slug>/` grouping is preserved, and `gh-stack` does not need to be told anything about LOOM.

### 4.2 One worker owns the whole stack

This is a deliberate departure from LOOM's "one agent per branch" norm and it is worth stating in plain terms: **for a stack epic, one worker is assigned, and that worker creates and operates on multiple branches.**

The rationale is forced by `gh-stack` itself. Three facts:

1. **Exit code 8 — "Stack is locked".** `gh stack` uses a worktree-local lock file to serialize stack mutations. The SKILL documents this under *"Exit codes and error recovery"*. Two workers sharing a stack's branches but running from different worktrees would hit this lock every time either tried to `add` or `rebase`. Two workers running from the same worktree are not a LOOM-supported topology.
2. **`git rerere` is worktree-local.** The SKILL's *"Rerere (conflict memory)"* note says: *"git rerere is enabled by init so previously resolved conflicts are auto-resolved in future rebases."* That cache lives in `.git/rr-cache` of the worktree doing the resolution. Spreading a stack across multiple worktrees throws the cache away.
3. **Stack files are not designed for concurrent writers.** `gh-stack` represents the stack via a file in the repo state; no locking discipline exists for cross-worktree access.

So the proposal commits to: one stack, one worker, one worktree. Layers are the worker's internal structure.

### 4.3 Scope enforcement across a stack — the union-scope model

LOOM's `protocol.md §6.1` rule reads:

> **Agent scope** — An agent may modify only files matching its `Scope` trailer. The orchestrator verifies at integration.

For stack-epic assignments, this is relaxed to a *union* scope model. The assignment's `Scope:` trailer is a union across every intended layer, and the orchestrator verifies at integration that every commit on every layer branch touches only files in the union.

Concretely, an assignment with `Scope: src/auth/**, src/api/**, src/frontend/**, tests/**` and layer positions `1:auth-middleware,2:api-endpoints,3:frontend` permits:

- commits on `loom/ratchet-auth-stack/auth-middleware` to touch any of `src/auth/**`, `src/api/**`, `src/frontend/**`, or `tests/**`.
- commits on `loom/ratchet-auth-stack/api-endpoints` to touch any of the same.
- commits on `loom/ratchet-auth-stack/frontend` to touch any of the same.

A stricter per-layer trailer is **optional and advisory**:

```
Scope-Layer: 1:src/auth/**,tests/auth/**
Scope-Layer: 2:src/api/**,tests/api/**
Scope-Layer: 3:src/frontend/**,tests/frontend/**
```

If present, the orchestrator warns on integration when a layer's commits exceed the advisory scope, but it does not reject the stack as long as the union scope holds. Advisory scope is for review readability, not enforcement.

### 4.4 Scope verification algorithm

The verification algorithm at integration is:

```python
def verify_stack_scope(stack_assignment):
    union = parse_scope(stack_assignment.trailers['Scope'])
    layers = parse_stack_position(stack_assignment.trailers['Stack-Position'])

    # 1. Enumerate all layer branches and verify they exist.
    for layer in layers:
        branch = f"loom/{stack_assignment.agent}-{stack_assignment.slug}/{layer.slug}"
        require_branch_exists(branch)

    # 2. Check every commit against the union scope.
    assigned_sha = stack_assignment.assigned_commit_sha
    for layer in layers:
        branch = f"loom/{stack_assignment.agent}-{stack_assignment.slug}/{layer.slug}"
        commits = git_commits_since(branch, assigned_sha)
        for commit in commits:
            for path in commit.files_changed:
                if not matches_any(path, union):
                    raise ScopeViolation(branch, commit.sha, path)

    # 3. Verify Stack-Op trailers form a valid gh-stack sequence.
    ops = collect_stack_ops_across_branches(layers, assigned_sha)
    validate_stack_op_sequence(ops)  # init first, then adds, then rebases/commits, then submit

    # 4. Verify Stack-Position trailer matches branch topology.
    for i, layer in enumerate(layers):
        parent = layers[i - 1].slug if i > 0 else 'main'
        parent_branch = 'main' if i == 0 else f"loom/{stack_assignment.agent}-{stack_assignment.slug}/{parent}"
        require_ancestor(parent_branch, branch)
```

Step 2 is where the "union" in "union scope" does its work: every commit on every branch is checked, just against the shared union rather than against each branch's own scope. This is *stricter* than per-branch scope in one sense (because every commit across every branch is checked) and *looser* in another (because the allowed path set is the union). The total surface area of "files a worker may touch" is exactly the same as the assignment's declared scope — the only thing that changes is how scope is partitioned across the worker's branches internally.

### 4.5 Edge case: a worker rebases a layer whose advisory scope it does not own

What if `ratchet` runs `gh stack down` to the `auth-middleware` layer and then accidentally commits a frontend change there?

**Answer:** the commit lands on the wrong branch, `gh stack rebase --upstack` replays it as part of the stack, and integration step 2 of §4.4 catches the scope violation — *only* if the edited path is outside the union. If the edited path is inside the union, then the worker has mis-partitioned its work (a content quality issue, not a security issue). The *advisory* Scope-Layer trailers emit a warning to help reviewers notice, but the commit is not rejected.

This is a deliberate choice. The enforcement boundary is the union — the worker's total authority envelope — and the within-stack partitioning is a hygiene concern rather than an invariant concern. If LOOM wants per-layer enforcement later, `Scope-Layer` can be promoted from advisory to enforced with a one-line config change.

### 4.6 Edge case: a worker force-pushes outside its stack namespace

What if a compromised or buggy worker runs `git push --force origin main`? Or more realistically, force-pushes `loom/moss-other-assignment/foo`?

**Answer:** integration-time verification enumerates only branches under `loom/<lead-agent>-<epic-slug>/*`. Any force-push outside that namespace is invisible to the stack-epic integrate step and must be caught by general LOOM invariants (branch-protection rules on `main`, orchestrator-only integration for other agents' branches, etc.). This proposal does not relax protections on branches outside the stack's namespace. The `worker-stack-driver` role grants force-push authority only within the stack namespace because `stack-worker-init` only calls `gh stack` against that namespace; a raw `git push --force` is not exposed by the tool. A worker bypassing the tool via raw shell is caught by the existing audit trail and rolled back at integration — see §5's failure modes.

---

## 5. Worker authority — the core section

This is the load-bearing section. Every other section of this proposal is downstream of the trust-boundary rethink here. This section was written first.

### 5.1 What workers gain the right to do

Workers assigned to a stack epic (i.e. whose ASSIGNED commit carries `Stack-Epic: true`) gain the right to run **every `gh stack` subcommand**, and only `gh stack` subcommands. Exhaustively:

| Command | Purpose |
|---------|---------|
| `gh stack init -p loom/<lead>-<epic> <bottom-layer>` | Create the stack |
| `gh stack add <layer-suffix>` | Add a layer on top of the current topmost |
| `gh stack rebase` | Rebase the whole stack on the latest trunk |
| `gh stack rebase --upstack` | Rebase everything from current layer upward |
| `gh stack rebase --downstack` | Rebase everything from trunk to current layer |
| `gh stack rebase --continue` | Resume after resolving conflicts |
| `gh stack rebase --abort` | Roll back to pre-rebase state |
| `gh stack sync` | Fetch, ff trunk, cascade rebase, push, sync PR state |
| `gh stack push` | Force-with-lease push all layer branches atomically |
| `gh stack submit --auto --draft` | Push and create draft PRs for every layer |
| `gh stack view --json` | Query stack topology and PR state |
| `gh stack checkout <branch>` | Navigate to a specific layer by name |
| `gh stack up`, `gh stack down` | Navigate one layer at a time |
| `gh stack top`, `gh stack bottom` | Navigate to extremes |
| `gh stack unstack` | Tear down the stack (for restructuring) |

Workers do **not** gain the right to run any non-`gh stack` GitHub write:

- `gh pr create` — forbidden. Draft PRs for the stack come from `gh stack submit --auto --draft` only.
- `gh pr merge` — forbidden. All merges go through orchestrator integration.
- `gh pr edit` — forbidden. PR metadata edits are orchestrator-only.
- `gh pr review`, `gh pr close`, `gh issue ...` — forbidden.
- Any raw `git push` to a branch outside `loom/<lead-agent>-<epic-slug>/*` — forbidden.
- Any `gh api` invocation — forbidden.

The grant is scoped to the `gh stack` subcommand tree, mediated through the `stack-worker-init` tool which both enforces "only `gh stack` ops run here" and stamps `Stack-Op:` trailers on the commits that follow. A worker invoking raw `gh stack` directly (bypassing the tool) produces commits without `Stack-Op:` trailers, which the integration step rejects.

### 5.2 Which LOOM invariants are relaxed (named, quoted, justified)

Each relaxation below quotes the exact invariant it affects, cites the file and section, and justifies why the relaxation is forced by the angle.

#### 5.2.1 `protocol.md §6.1` — "Workspace write"

**Original text:**
> **Workspace write** — Only the orchestrator writes to the workspace. Agents MUST NOT.

**Relaxation:** Workers do not write to the *primary* workspace (that rule is preserved — workers still commit only inside their worktrees). What changes is that *force-pushes to remote refs under `loom/<lead-agent>-<epic-slug>/*` are now permitted for `worker-stack-driver` role*. Previously those remote refs were part of the orchestrator's exclusive write surface.

**Justification:** `gh stack rebase` rewrites history and force-pushes by design. The SKILL's `gh stack push` entry says *"Pushes all active (non-merged) branches atomically (`--force-with-lease --atomic`)"*. Forcing this through the orchestrator means the orchestrator would need to copy the worker's `.git/rr-cache` over (which leaks worktree internals), re-drive the rebase, and hold state across possibly multi-step conflict resolution. All of that is possible in principle but makes context compaction fatal: if the orchestrator loses mid-rebase state, it cannot recover without the worker's `rerere` cache, and LOOM has no protocol for shipping caches across process boundaries.

The relaxation is bounded: workers force-push *only* to refs under their epic's namespace, and only through the `stack-worker-init` tool (which only calls `gh stack` subcommands). A worker that bypasses the tool with raw `git push --force` is caught by the integration-time audit check, which enumerates expected force-push events from `Stack-Op:` trailers and rejects any branch whose reflog includes unexpected force-pushes.

#### 5.2.2 `protocol.md §6.1` — "Agent scope"

**Original text:**
> **Agent scope** — An agent may modify only files matching its `Scope` trailer. The orchestrator verifies at integration.

**Relaxation:** The `Scope:` trailer for a stack-epic assignment is a *union* across all layers. The orchestrator verifies at integration that every commit on every layer branch touches only files in the union — the verification algorithm in §4.4 is stricter than today in that it checks every commit on every branch, but it is looser in that the target set is the union rather than a per-branch subset.

**Justification:** `gh stack` is fundamentally a mechanism for *partitioning* work across branches. A worker cannot know in advance which files belong to which layer without having done the work; forcing per-layer scope upfront means either the worker rejects its own assignment or the orchestrator has to re-issue scope on every mid-stack edit. The union model accepts the partition as a worker-side decision while preserving the total envelope of "which files the assignment is allowed to touch." Nothing the worker can do expands the total envelope.

#### 5.2.3 `protocol.md §3.3` — `integrate()` `--no-ff` merge

**Original text:** the `integrate()` procedure in `protocol.md §3.3` reads (paraphrasing, since the exact text is in the protocol file):
> The orchestrator merges an agent's branch into the workspace. Integration is sequential and atomic.

with the implicit rule that the merge is `--no-ff` so the merge commit records the integration in the first-parent history. This is the clause being relaxed.

**Relaxation:** For stack-epic branches, `integrate()` is replaced with a **cascading fast-forward merge**. The orchestrator runs, in order:

```sh
git checkout main
git merge --ff-only loom/<lead>-<epic>/<layer-1>   # bottom
git merge --ff-only loom/<lead>-<epic>/<layer-2>
git merge --ff-only loom/<lead>-<epic>/<layer-3>   # top
```

No merge commits are created. Every integrated commit is a worker commit with `Agent-Id`, `Session-Id`, and `Stack-Op` trailers.

**Justification:** `gh stack` rebases and force-pushes by design. After `gh stack submit --auto --draft`, the stack's branches have been rebased on top of `main` and force-pushed. The original ASSIGNED commit is no longer an ancestor of the worker's branches — it was rewritten out of history during the first rebase. `--no-ff` merge requires the ASSIGNED commit to be a parent of the branch being merged, which is no longer true. So `--no-ff` is structurally impossible for stack-epic branches unless LOOM refuses to let workers rebase, which is exactly the "convention-only recipe" rejected alternative.

The relaxation is *audit-preserving*, not *audit-destroying*. See §5.3.

#### 5.2.4 `pr-create.ts` — `roles: ['orchestrator']` precedent

**Source:** `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts` line 27.

**Original state:** `roles: ['orchestrator']`. Every PR creation goes through the orchestrator.

**Relaxation:** the existing guard on `pr-create` stays in force. The new `stack-worker-init` tool is registered with `roles: ['worker-stack-driver']`, a role that does not yet exist in `repos/bitswell/loom-tools/src/types/tool.ts` and which this proposal adds. The role is a strict superset of `worker` — it grants worker-level authority plus the `gh stack` subcommand tree.

**Justification:** introducing a new role tier is the minimum change that lets the type system express the new trust boundary. Widening `worker` itself would grant stack authority to every worker, which violates least-privilege; leaving `worker` alone and using `orchestrator` for `stack-worker-init` just reproduces the MCP-wrapper alternative. A new role is the only thing that captures "elevated worker, but only for stack epics."

#### 5.2.5 `worker-template.md §9` — "Scope Enforcement"

**Original text:**
> **9. Scope Enforcement**
>
> You MUST NOT modify files outside `scope.paths_allowed` in AGENT.json. The orchestrator verifies scope at integration and will reject the branch if any commit touches a file outside scope.

**Relaxation:** overridden by the new §10 added in §3.3 of this proposal. For stack-epic workers, `scope.paths_allowed` is the union across layers, and the verification is performed against the entire set of layer branches rather than a single branch.

**Justification:** the text of §9 is unchanged for non-stack workers. Stack-epic workers read §10, which explicitly redefines the enforcement envelope. There is no interpretation in which §9 and §10 conflict — §10 applies only when `Stack-Epic: true` is in the ASSIGNED commit, and §9 applies otherwise.

#### 5.2.6 `protocol.md §2` — implicit one-agent-per-branch rule

**Original text:** `protocol.md §2` describes the LOOM state machine in terms of *"one worker per assignment"*, and elsewhere in the protocol the assumption is that each assignment is paired with exactly one branch `loom/<agent>-<slug>`.

**Relaxation:** one worker per stack-epic assignment is still preserved — the stack is owned by a single worker. The relaxation is that the worker is associated with multiple branches (the layer branches) under the same assignment. The ASSIGNED commit declares the stack by carrying `Stack-Epic: true` and listing the layer slugs in `Stack-Position:`.

**Justification:** `gh-stack` requires one agent to drive a stack (see §4.2). Splitting the stack across multiple workers hits exit code 8 on every shared operation. The one-worker constraint is therefore forced, and allowing one worker to hold multiple branches is the minimum accommodation that lets LOOM describe what actually happens on disk.

### 5.3 How scope enforcement still works when workers touch branches beyond their own

A stack-epic worker touches multiple branches. "Branches beyond their own" is a misleading framing in this context — *all* the layer branches are the worker's own, because the worker is the sole owner of the stack. The scope envelope is the union; the verification algorithm is the one in §4.4; the failure modes are enumerated in §5.6.

The sharper framing is: *what stops a stack-epic worker from force-pushing garbage onto another agent's branch?* Three mechanisms:

1. **Tool scope.** `stack-worker-init` only runs `gh stack` subcommands against the stack identified by the active assignment. It does not expose raw `git push`, so there is no code path that force-pushes to an unrelated branch via the tool.
2. **Namespace enforcement at integration.** The orchestrator enumerates only branches under `loom/<lead-agent>-<epic-slug>/*` when integrating the stack. Any other branch the worker may have touched is invisible to this integration and must be caught by general LOOM invariants (branch protection, orchestrator-only integration for other workers' branches, etc.).
3. **Audit reconstruction.** Every legitimate force-push corresponds to a `Stack-Op: rebase` or `Stack-Op: submit` commit on one of the layer branches. At integration, the orchestrator walks `reflog` on each layer branch and cross-checks that every force-push event has a matching `Stack-Op` trailer. Unexpected force-pushes are rejected.

### 5.4 How the audit trail is preserved

LOOM's existing audit trail works by `--no-ff` merge commits on `main`: `git log --first-parent main` gives you a sequence of merge commits, each of which was an integration event, and each of those merge commits references an ASSIGNED commit via its non-first parent. This is the invariant §5.2.3 relaxes.

The replacement is *trailer-based audit*. Every worker commit on every layer branch carries:

- `Agent-Id: <worker>` — who made the commit
- `Session-Id: <uuid>` — which work session
- `Stack-Op: <op>` — what `gh stack` operation the commit represents
- `Heartbeat: <timestamp>` — when the commit was made

After cascading `--ff-only` integration, every one of those commits is now on `main` as a direct ancestor (not behind a merge commit). Reconstructing the audit trail becomes a `git log` query:

```sh
# Reconstruct the sequence of stack operations on main
git log --first-parent main \
  --format='%H %cI %s%n%(trailers:key=Agent-Id,key=Session-Id,key=Stack-Op)%n---'
```

Sample output (abbreviated from the §6 end-to-end example):

```
e5f1a21 2026-04-14T10:00:00Z chore(loom): begin auth-stack
Agent-Id: ratchet
Session-Id: c12e0e01-...
Stack-Op: init
---
a3b7c82 2026-04-14T10:04:11Z feat(auth): add middleware core
Agent-Id: ratchet
Session-Id: c12e0e01-...
Stack-Op: commit
---
d9f2118 2026-04-14T10:07:32Z chore(loom): stack add api-endpoints
Agent-Id: ratchet
Session-Id: c12e0e01-...
Stack-Op: add
---
... etc ...
---
72ab3f1 2026-04-14T11:42:17Z feat(auth-stack): submit draft stack
Agent-Id: ratchet
Session-Id: c12e0e01-...
Stack-Op: submit
```

Compare this with the `--no-ff` merge-commit audit: that format tells you *when a branch landed*, but nothing about *when commits were rebased, when conflicts were recovered, or when layers were reordered mid-work*. Those are rewritten-history events that `--no-ff` cannot represent because they happened before the merge. The trailer-based audit *does* represent them, because every rebase step is a `Stack-Op: rebase` commit with its own timestamp.

In other words: the trailer-based audit is strictly *more informative* than `--no-ff` merge structure. It records the full history of the work, including the rebases, not just the final landing event. The reviewer of this proposal should evaluate whether that is a fair trade for the loss of merge-commit structure on `main`. This proposal asserts that it is, because the thing LOOM's audit trail is *for* is reconstructing what a worker did during the work — and rebases are part of the work.

### 5.5 The new trust tier: `worker-stack-driver`

Introduce the concept explicitly: `worker-stack-driver` is a new role tier in `repos/bitswell/loom-tools/src/types/tool.ts`. It is a strict superset of `worker` with one additional privilege: the authority to run the `gh stack` subcommand tree from within the worker's worktree, and to force-push to remote refs within the stack's `loom/<lead-agent>-<epic-slug>/*` namespace.

This privilege is granted **only** when the worker's active assignment carries `Stack-Epic: true`. A worker receiving a regular (non-stack) assignment remains on the old `worker` tier and cannot call `stack-worker-init`. The MCP dispatch layer checks this on every tool call.

The orchestrator's ASSIGNED commit is the sole point that grants this privilege. There is no other code path by which a worker becomes `worker-stack-driver`. An orchestrator that does not issue `Stack-Epic: true` can be confident no worker will ever run `gh stack`.

### 5.6 The orchestrator's retained authority

The orchestrator retains exclusive authority over:

1. **Assignment creation.** Every ASSIGNED commit is orchestrator-only. Workers cannot self-promote into `worker-stack-driver`.
2. **Stack-epic classification.** The decision of whether an epic is a stack epic belongs to the orchestrator alone. It is encoded in the `Stack-Epic: true` trailer on the ASSIGNED commit.
3. **Scope verification at integration.** The algorithm in §4.4 is run by the orchestrator. Workers do not verify their own scope.
4. **Cascading `--ff-only` integration.** The orchestrator runs the cascading merge into `main`. Workers never touch `main`.
5. **Non-stack PR creation via `pr-create`.** Workers have no authority to create non-stack PRs.
6. **PR promotion, merging, and closing on GitHub.** `gh pr merge`, `gh pr close`, `gh pr edit` remain orchestrator-only. The draft PRs workers create via `gh stack submit --auto --draft` are promoted and merged by the orchestrator after integration succeeds.
7. **The audit contract.** The orchestrator owns the rule that every integrated commit must carry `Agent-Id`, `Session-Id`, and `Stack-Op` trailers. Workers that fail to stamp trailers are rejected at integration.

The summary: the orchestrator owns **contracts** (assignment, scope, integration, audit). The worker owns **mechanics** (commits, branches, rebases, submission). This is the trust-boundary rethink, stated precisely.

### 5.7 The answer to the trust-model question

*What happens to LOOM's trust model when we let workers drive stacking?*

**The trust model shifts from mechanism control to contract control. The orchestrator stops being a bottleneck for mechanical operations and becomes a verifier of worker outputs. The orchestrator owns the scope contract, the integration contract, and the audit contract; workers own the mechanics within those contracts. Any operation that is purely mechanical — rebasing, force-pushing within a bounded namespace, submitting draft PRs — is delegated to the worker because the worker is the party holding the working tree, the rerere cache, and the conflict-resolution context. Any operation that touches the global state of `main`, the GitHub merge queue, or the cross-worker state of the repo remains orchestrator-only because those operations require cross-agent authority the worker does not have.**

This paragraph is the closing of §5 and should be read as the single-paragraph version of the entire proposal.

### 5.8 Failure modes

Four failure modes the proposal must address explicitly:

#### 5.8.1 A worker force-pushes outside its stack namespace

**Scenario:** compromised or buggy worker runs `git push --force origin main` or force-pushes to some other agent's branch.

**Detection:** integration-time verification enumerates only branches under `loom/<lead-agent>-<epic-slug>/*`. Any out-of-namespace push is invisible to stack integration and must be caught by general LOOM invariants — branch protection on `main`, orchestrator-only integration for other agents' branches, and `gh` auth scopes on the worker's token.

**Mitigation:** the `stack-worker-init` tool does not expose raw `git push`. A worker invoking raw `git push --force` from shell is caught by the audit reconstruction (§5.3): every legitimate force-push corresponds to a `Stack-Op: rebase` or `Stack-Op: submit` commit, and unexpected reflog entries are rejected.

#### 5.8.2 A worker runs `gh stack submit --auto` without `--draft`

**Scenario:** the worker creates non-draft PRs, bypassing the orchestrator's exclusive authority over promoting drafts to ready.

**Detection:** integration rejects any branch under the stack namespace whose PR is non-draft unless the orchestrator itself promoted it. The orchestrator queries `gh stack view --json` before integrating and fails loudly if any PR is in an unexpected state.

**Mitigation:** `stack-worker-init` refuses to run `submit` without `--draft`. A worker bypassing via raw `gh stack submit --auto` is caught at detection.

#### 5.8.3 A worker invokes `gh pr create` instead of `gh stack submit`

**Scenario:** worker shell-invokes `gh pr create` to make a non-stack PR under the epic namespace.

**Detection:** integration enumerates all PRs under the epic namespace and requires every one to have been created by `gh stack submit` (identifiable by `gh stack view --json`'s PR metadata — stacked PRs are linked together via GitHub's stacks feature, non-stacked PRs are not).

**Mitigation:** the `stack-worker-init` tool does not expose `pr create`; the raw-shell bypass is caught at integration. Additionally, the epic's branch-protection ruleset can be configured to require stack-created PRs only (a repo-level config, not a protocol change).

#### 5.8.4 A worker gets compacted mid-rebase

**Scenario:** the worker is in the middle of `gh stack rebase --continue` when context compaction happens. State on disk: partially resolved conflicts, `git rerere` cache populated, `REBASE_HEAD` set.

**Recovery:** the SKILL's "Handle rebase conflicts" workflow is explicitly designed to be recoverable from disk state. Worker template §10 prescribes the recovery procedure: re-read the worktree state, check for `REBASE_HEAD`, parse any outstanding conflicted files, resolve them, and run `gh stack rebase --continue`. The `rerere` cache makes most re-runs of previously-resolved conflicts automatic.

**Audit:** even a compacted-and-recovered worker leaves a trail of `Stack-Op: rebase` and `Stack-Op: rebase-continue` commits in the layer branches' reflogs. The orchestrator can reconstruct the full sequence from trailers at integration.

---

## 6. End-to-end example

The canonical three-layer epic, traced concretely. The worker is `ratchet`. The epic is a three-layer auth feature: `auth-middleware → api-endpoints → frontend`.

### 6.1 Epic decomposition (orchestrator)

The orchestrator identifies the epic as a stack epic and emits a single ASSIGNED commit on `loom/ratchet-auth-stack`:

```
task(ratchet): stack epic — auth middleware → api endpoints → frontend

Three-layer stack. Auth middleware is foundational (touches src/auth/**),
api-endpoints consumes the middleware (src/api/**), frontend consumes the
api (src/frontend/**). Shared test coverage in tests/**.

Each layer should be its own reviewable PR. Use gh stack for layering.

Agent-Id: bitswell
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: auth-stack
Stack-Epic: true
Stack-Position: 1:auth-middleware,2:api-endpoints,3:frontend
Scope: src/auth/**, src/api/**, src/frontend/**, tests/**
Dependencies: none
Budget: 120000
Epic: #74
```

Only *one* branch is created at assignment time — `loom/ratchet-auth-stack`. The worker will create the layer branches itself via `gh stack add`.

### 6.2 Worker start

The orchestrator dispatches `ratchet` into the worktree `loom/ratchet-auth-stack`. The worker reads the ASSIGNED commit, sees `Stack-Epic: true`, loads `worker-template.md §10`, and begins:

```sh
# worker's first action: IMPLEMENTING state commit with Stack-Op: init
git commit --allow-empty -m "$(cat <<'EOF'
chore(loom): begin auth-stack

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:00:00Z
Stack-Op: init
EOF
)"

# initialize the stack with the bottom layer
gh stack init -p loom/ratchet-auth-stack auth-middleware
# → creates loom/ratchet-auth-stack/auth-middleware and checks it out
```

### 6.3 Layer 1 — `auth-middleware`

Ratchet writes the auth middleware, commits it:

```sh
# write src/auth/middleware.ts and tests/auth/middleware.test.ts
git add src/auth/middleware.ts tests/auth/middleware.test.ts
git commit -m "$(cat <<'EOF'
feat(auth): add middleware core

Signature validation, token parsing, request.auth binding.

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:04:11Z
Stack-Op: commit
EOF
)"

# more commits on the same layer as needed
git add src/auth/types.ts
git commit -m "$(cat <<'EOF'
feat(auth): add shared types

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:06:47Z
Stack-Op: commit
EOF
)"
```

### 6.4 Layer 2 — `api-endpoints`

Ratchet adds the next layer and writes the API code:

```sh
gh stack add api-endpoints
# → creates loom/ratchet-auth-stack/api-endpoints on top of auth-middleware

git commit --allow-empty -m "$(cat <<'EOF'
chore(loom): stack add api-endpoints

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:07:32Z
Stack-Op: add
EOF
)"

# write src/api/users.ts, src/api/sessions.ts, tests/api/...
git add src/api/users.ts src/api/sessions.ts tests/api/users.test.ts tests/api/sessions.test.ts
git commit -m "$(cat <<'EOF'
feat(api): user and session endpoints

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:21:03Z
Stack-Op: commit
EOF
)"
```

### 6.5 Mid-stack edit

While writing the API layer, ratchet realises the middleware needs a new helper (`requireAuth(scope)`). This is the operation that `gh stack` exists for — and the operation that every non-worker-driven integration cannot handle.

```sh
# navigate down to the middleware layer
gh stack down
# → now on loom/ratchet-auth-stack/auth-middleware

git commit --allow-empty -m "$(cat <<'EOF'
chore(loom): stack down to auth-middleware

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:23:44Z
Stack-Op: down
EOF
)"

# add the helper
git add src/auth/middleware.ts
git commit -m "$(cat <<'EOF'
feat(auth): add requireAuth(scope) helper

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:24:19Z
Stack-Op: commit
EOF
)"

# rebase upstack so api-endpoints picks up the helper
gh stack rebase --upstack
# → rebases loom/ratchet-auth-stack/api-endpoints onto the updated auth-middleware

git commit --allow-empty -m "$(cat <<'EOF'
chore(loom): rebase upstack after middleware helper

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:25:12Z
Stack-Op: rebase
EOF
)"

# back to the top
gh stack top
# → now on loom/ratchet-auth-stack/api-endpoints (still top of stack so far)
```

At this point the commit graph looks like:

```
main
 └── loom/ratchet-auth-stack/auth-middleware   (3 commits: middleware core, types, requireAuth helper)
      └── loom/ratchet-auth-stack/api-endpoints (1 commit: user + session endpoints, rebased)
```

### 6.6 Layer 3 — `frontend`

```sh
gh stack add frontend
# → creates loom/ratchet-auth-stack/frontend

git commit --allow-empty -m "$(cat <<'EOF'
chore(loom): stack add frontend

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:28:01Z
Stack-Op: add
EOF
)"

# write src/frontend/auth-form.tsx, src/frontend/session-manager.ts, tests/frontend/...
git add src/frontend/auth-form.tsx src/frontend/session-manager.ts tests/frontend/auth-form.test.tsx
git commit -m "$(cat <<'EOF'
feat(frontend): auth form and session manager

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:48:56Z
Stack-Op: commit
EOF
)"
```

### 6.7 Conflict recovery

Ratchet realises the session-manager also needs a change in the API layer. Navigate down, edit, rebase — but this time a conflict hits during the rebase:

```sh
gh stack checkout loom/ratchet-auth-stack/api-endpoints
# → navigate directly to the API layer

git add src/api/sessions.ts
git commit -m "$(cat <<'EOF'
feat(api): add /sessions/refresh endpoint

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:54:03Z
Stack-Op: commit
EOF
)"

gh stack rebase --upstack
# → fails with exit code 3, conflict in src/frontend/session-manager.ts
```

SKILL's "Handle rebase conflicts" workflow:

```sh
# read the conflicted file, resolve markers, stage
git add src/frontend/session-manager.ts

gh stack rebase --continue
# → exit 0

git commit --allow-empty -m "$(cat <<'EOF'
chore(loom): rebase-continue after session-manager conflict

Resolved markers in src/frontend/session-manager.ts. rerere
cached the resolution.

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T10:57:41Z
Stack-Op: rebase-continue
EOF
)"
```

### 6.8 Submit

```sh
gh stack top
gh stack submit --auto --draft
# → pushes loom/ratchet-auth-stack/{auth-middleware,api-endpoints,frontend}
# → creates three draft PRs
# → links them as a stack on GitHub

git commit --allow-empty -m "$(cat <<'EOF'
feat(auth-stack): submit draft stack

Three-layer auth feature, draft PRs:

- #210 loom/ratchet-auth-stack/auth-middleware (base: main)
- #211 loom/ratchet-auth-stack/api-endpoints (base: auth-middleware)
- #212 loom/ratchet-auth-stack/frontend (base: api-endpoints)

Agent-Id: ratchet
Session-Id: c12e0e01-1a2b-4c3d-9e8f-011223344556
Task-Status: COMPLETED
Files-Changed: 12
Key-Finding: three draft PRs created, stack linked on GitHub
Key-Finding: rerere cache caught one mid-rebase conflict in session-manager
Heartbeat: 2026-04-14T11:42:17Z
Stack-Op: submit
EOF
)"
```

At this point the worker is done. Ratchet does not promote the drafts, does not merge, does not edit PR metadata. Ratchet returns to the orchestrator.

### 6.9 Integration phase (orchestrator)

The orchestrator runs scope verification (algorithm in §4.4):

1. Enumerate layer branches: `loom/ratchet-auth-stack/{auth-middleware,api-endpoints,frontend}`. All exist. ✓
2. Check every commit against union scope `src/auth/**, src/api/**, src/frontend/**, tests/**`. Middleware commits touch `src/auth/**` and `tests/auth/**` — in scope. API commits touch `src/api/**` and `tests/api/**` — in scope. Frontend commits touch `src/frontend/**` and `tests/frontend/**` — in scope. ✓
3. Verify `Stack-Op:` sequence: `init, commit, commit, add, commit, down, commit, rebase, add, commit, commit, rebase-continue, submit`. Valid `gh-stack` sequence. ✓
4. Verify branch topology: api-endpoints has auth-middleware as ancestor, frontend has api-endpoints as ancestor. ✓

Then cascading `--ff-only` merge:

```sh
git checkout main
git pull --ff-only
git merge --ff-only loom/ratchet-auth-stack/auth-middleware
git merge --ff-only loom/ratchet-auth-stack/api-endpoints
git merge --ff-only loom/ratchet-auth-stack/frontend
git push origin main
```

Each merge is a fast-forward — no merge commit is created. The worker's commits are now direct ancestors of `main`.

Finally, the orchestrator handles the GitHub PR state:

- **Option A (auto-merge on GitHub):** promote each draft to ready (`gh pr ready`), merge in order via the merge queue. This preserves the PRs as visible artifacts but duplicates the local fast-forward.
- **Option B (close-in-favor-of-push):** close each draft with a comment pointing to the fast-forwarded commits on `main`. This is cleaner on the local history but costs the PR audit trail on GitHub.

This proposal picks **Option A**. The reason: PRs on GitHub are the review artifact reviewers actually look at, and closing them in favor of a direct push loses the review discussion. The cost of Option A is that the local `main` now has the commits before GitHub's merge queue processes them, so the merge queue has to no-op. For auto-merge queues this is acceptable; for strict linear-history queues it is not, in which case flip to Option B.

### 6.10 Audit reconstruction

After integration, the full worker sequence is reconstructable from `main`:

```sh
git log --first-parent main \
  --format='%h %cI %s%n  %(trailers:key=Agent-Id,key=Stack-Op,separator=%x20)%n'
```

Sample output:

```
e5f1a21 2026-04-14T10:00:00Z chore(loom): begin auth-stack
  Agent-Id: ratchet Stack-Op: init

a3b7c82 2026-04-14T10:04:11Z feat(auth): add middleware core
  Agent-Id: ratchet Stack-Op: commit

72e4f19 2026-04-14T10:06:47Z feat(auth): add shared types
  Agent-Id: ratchet Stack-Op: commit

d9f2118 2026-04-14T10:07:32Z chore(loom): stack add api-endpoints
  Agent-Id: ratchet Stack-Op: add

b1c8923 2026-04-14T10:21:03Z feat(api): user and session endpoints
  Agent-Id: ratchet Stack-Op: commit

a8eb071 2026-04-14T10:23:44Z chore(loom): stack down to auth-middleware
  Agent-Id: ratchet Stack-Op: down

5f0a9c2 2026-04-14T10:24:19Z feat(auth): add requireAuth(scope) helper
  Agent-Id: ratchet Stack-Op: commit

4f1a803 2026-04-14T10:25:12Z chore(loom): rebase upstack after middleware helper
  Agent-Id: ratchet Stack-Op: rebase

3c9d217 2026-04-14T10:28:01Z chore(loom): stack add frontend
  Agent-Id: ratchet Stack-Op: add

e7b2541 2026-04-14T10:48:56Z feat(frontend): auth form and session manager
  Agent-Id: ratchet Stack-Op: commit

8d91f03 2026-04-14T10:54:03Z feat(api): add /sessions/refresh endpoint
  Agent-Id: ratchet Stack-Op: commit

21f7b1a 2026-04-14T10:57:41Z chore(loom): rebase-continue after session-manager conflict
  Agent-Id: ratchet Stack-Op: rebase-continue

72ab3f1 2026-04-14T11:42:17Z feat(auth-stack): submit draft stack
  Agent-Id: ratchet Stack-Op: submit
```

Every stack op is captured. Every rebase is captured. Every conflict recovery is captured. The reviewer can replay the entire work session from the trailer trail, including events that `--no-ff` merge-commit structure *cannot* represent (the mid-work rebases).

---

## 7. Risks and rejected alternatives

### 7.1 Risks

**Risk 1 — a stack-epic worker misbehaves with elevated authority.** The worker has force-push authority within its namespace and draft-PR authority on GitHub. A malicious or buggy worker could in principle force-push garbage, submit non-draft PRs, or attempt to reach outside its namespace.

*Mitigation:* integration-time scope verification against union scope catches content violations. `Stack-Op:` sequence validation catches protocol violations. Mandatory draft-only submission (enforced by `stack-worker-init` and re-checked at integration) catches ready-state violations. The attack surface is bounded to the worker's own stack namespace — the orchestrator still owns merges and cross-agent state.

**Risk 2 — audit trail depends on trailer discipline.** If a worker forgets to stamp `Stack-Op:` trailers, the audit reconstruction loses fidelity. A silently dropped trailer might mask a rebase that changed semantics.

*Mitigation:* `stack-worker-init` auto-stamps `Stack-Op:` trailers on the commit following every stack operation. Workers cannot run `gh stack` except through the tool (enforced by the `worker-stack-driver` role guard). A worker running raw `gh stack` from shell is caught at integration by the mismatched reflog: force-push events without corresponding `Stack-Op:` commits are rejected.

**Risk 3 — one-worker-per-stack ceiling on parallelism.** Because the stack is serialized by construction (§4.2), a stack epic cannot be split across parallel workers. An epic with three layers is bound to one worker's throughput.

*Mitigation:* this is a property of `gh-stack` itself (strictly linear stacks, worktree-local `rerere`, exit code 8), not of this proposal. Parallel epics that genuinely have independent dimensions should be split into separate stacks, each with its own worker. The "N parallel workers on N independent stacks" topology is supported.

**Risk 4 — relaxing `--no-ff` weakens the audit invariant for stack-epic branches.** `main`'s first-parent history no longer has merge commits for integrations. Tools that walk `--first-parent` looking for "integration events" will find direct worker commits instead.

*Mitigation:* this is a *deliberate trade*, not a concession. The replacement trailer-based audit is strictly more informative (§5.4). If a downstream tool assumes `--no-ff` structure, it must be updated to read trailers — which is a smaller change than losing `gh-stack` integration entirely. This proposal names the cost explicitly because pretending it is free would be dishonest and would damage the review process.

**Risk 5 — worker context compaction during multi-step rebase.** A long rebase with multiple conflicts can span more work than a worker's context window, and compaction can happen mid-flow.

*Mitigation:* SKILL's conflict workflow is explicitly designed to be recoverable from disk state. `REBASE_HEAD`, the `rerere` cache, and outstanding conflicted-file markers survive compaction. Worker template §10 prescribes the recovery procedure; the `rerere` cache means previously-resolved conflicts auto-resolve on re-run.

**Risk 6 — GitHub stacks require repo-level feature enablement.** `gh stack submit --auto --draft` only links PRs as a stack on GitHub if the repo has stacks enabled.

*Mitigation:* the stack-epic recipe (§3.3) includes a pre-flight check that verifies the repo has stacks enabled before issuing the ASSIGNED commit. If the repo does not, the orchestrator declines to decompose the epic as a stack epic and falls back to a non-stack decomposition.

### 7.2 Rejected alternatives

#### Rejected A — post-hoc projection (team 1's angle)

**Mechanism:** workers commit a linear stream of work; the orchestrator projects a stack onto the commits after integration by cherry-picking into layered branches.

**Why rejected:** post-hoc projection cannot handle mid-work conflict recovery or mid-stack edits, because the stack does not exist until after the work is done. Exactly the operations that justify stacking in the first place — navigating down to change a lower layer, rebasing upstack, resolving conflicts with `rerere` — are unavailable during the work. It solves the audit-trail problem (`--no-ff` is preserved by doing the integration first and then projecting) at the cost of the conflict-recovery problem. This proposal makes the opposite bet: pay the audit-structure cost to buy worker-driven conflict recovery, because conflict recovery is the thing that `gh-stack` is *for*.

A secondary issue with post-hoc projection: it requires the orchestrator to impersonate the worker at the moment it creates the projected layers. GitHub sees the PRs as orchestrator-authored even though the commits are worker-authored. This damages the review audit on GitHub itself. Worker-side stacking keeps authorship honest because the worker is actually the author.

#### Rejected B — orchestrator-only `gh stack` via MCP wrapper

**Mechanism:** a new `stack-create`, `stack-add`, `stack-rebase`, `stack-submit` set of MCP tools, all with `roles: ['orchestrator']`. Workers commit content; the orchestrator wraps every stack op.

**Why rejected:** every stack op becomes a round-trip through the orchestrator, which has no working tree and no `rerere` cache. Mid-rebase state (`REBASE_HEAD`, conflicted files, `rerere` cache) cannot be transferred across process boundaries without leaking `.git` internals, which LOOM's protocol forbids. Conflict recovery serializes on a process that may be compacted in the middle of the rebase, which silently destroys mid-flow state. And the orchestrator's latency per op adds up over a realistic stack's lifetime: a single stack epic with three layers, two mid-stack edits, and one rebase conflict generates ~20 `gh stack` invocations, each of which would be an orchestrator round-trip in this model.

The deep issue is that `gh-stack` was designed for local-first, worktree-resident operation. Wrapping it in an RPC that strips it of its working tree defeats the point.

#### Rejected C — keep orchestrator-only but pre-create all stack branches upfront

**Mechanism:** the orchestrator runs `gh stack init` and `gh stack add` for every layer before the worker starts. The worker just commits to the pre-created branches. The orchestrator runs `gh stack submit` at the end.

**Why rejected:** it cannot handle *any* mid-stack edit. If the worker on layer 3 needs to change layer 1, it has no tools to navigate down and rebase upstack — those are `gh stack` commands, which this alternative forbids. The worker either commits the change on the wrong layer (content ends up in the wrong PR), or gives up and asks the orchestrator to do it (which is rejected alternative B), or rejects its own assignment. This alternative works only for stacks that do not need mid-stack edits, which is approximately zero real stacks.

Additionally: `gh stack submit` at the end assumes no conflicts. Any upstream move in `main` during the work invalidates the stack, and the orchestrator has no way to re-rebase without restarting the whole alternative-B round-trip dance.

#### Rejected D — full `gh` grant to all workers (not just stack-epic)

**Mechanism:** widen the `worker` role to include the entire `gh` CLI. Drop the `worker-stack-driver` tier — every worker can run `gh stack`, `gh pr`, `gh issue`, etc.

**Why rejected:** massively wider blast radius for no benefit on non-stack assignments. A non-stack worker has no legitimate reason to run `gh pr create` (that is `pr-create`'s job) or `gh issue create` or `gh api`. Widening the grant unnecessarily violates least-privilege. The `worker-stack-driver` role exists precisely to scope the grant to the set of assignments that actually need it.

A related rejection: "grant `gh stack` but not other `gh` subcommands to the `worker` role globally, without a new role tier." This is rejected because the MCP tool system's enforcement unit is the tool, not the subcommand. Expressing "this worker can run `gh stack` but not `gh pr create`" requires either two separate tools (which is what this proposal does) or a subcommand-level allowlist inside a monolithic `gh` tool (which is a much larger design change). The new role tier is cheaper and clearer.

#### Rejected E — per-layer workers sharing a worktree via file locks

**Mechanism:** N workers, each assigned to one layer, sharing a single worktree serialized by an external lock.

**Why rejected:** LOOM's worker template assumes a worker owns its worktree exclusively — there is no sharing protocol, no lock, and no coordination primitives between workers. Introducing file-level locking is a protocol-wide change for a single-feature benefit. And the benefit is marginal because `gh stack`'s exit code 8 and `rerere`'s worktree-local cache would *still* force serialization; adding a file lock on top just reproduces §4.2's "one worker owns the stack" conclusion with more machinery.

---

## Appendix A — Referenced files

All paths absolute unless marked `repo:`.

- **Epic RFP:** `gh issue view 74 --repo bitswell/bitswell`
- **gh-stack SKILL:** `/home/willem/.agents/skills/gh-stack/SKILL.md`
- **LOOM protocol:** `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` (§2 state machine, §3.3 integrate, §6.1 security)
- **LOOM schemas:** `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` (§3.2 state trailers, §3.3 assignment trailers, §4.1 required-per-state)
- **LOOM worker template:** `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/worker-template.md` (§4 Do the Work, §9 Scope Enforcement, proposed new §10)
- **pr-create (role precedent):** `repo: repos/bitswell/loom-tools/src/tools/pr-create.ts` line 27
- **pr-retarget (unchanged):** `repo: repos/bitswell/loom-tools/src/tools/pr-retarget.ts`
- **dag-check (reused):** `repo: repos/bitswell/loom-tools/src/tools/dag-check.ts`
- **Tool type (role enum amended):** `repo: repos/bitswell/loom-tools/src/types/tool.ts`
- **Tool index (new registration):** `repo: repos/bitswell/loom-tools/src/tools/index.ts`
- **New tool file:** `repo: repos/bitswell/loom-tools/src/tools/stack-worker-init.ts` (proposed)

## Appendix B — The one-paragraph version

For readers who want the whole proposal in one paragraph:

> This proposal delegates `gh stack` authority to workers for stack-epic assignments. A new `worker-stack-driver` role is introduced in `repos/bitswell/loom-tools/src/types/tool.ts`, a new `stack-worker-init` MCP tool is added in `repos/bitswell/loom-tools/src/tools/`, and the worker template gains a §10 that relaxes workspace-write and scope-enforcement invariants for `Stack-Epic: true` assignments. Workers run `gh stack init/add/rebase/submit` from their worktrees, own their stack's `loom/<lead>-<epic>/*` namespace, and submit draft PRs only. Integration is cascading `--ff-only` merge into `main`, with audit preserved by `Stack-Op:` / `Agent-Id:` / `Session-Id:` / `Heartbeat:` trailers rather than by `--no-ff` merge-commit structure. The trust model shifts from *the orchestrator owns every GitHub write* to *the orchestrator owns the assignment/scope/integration/audit contracts; workers own the stack mechanics within those contracts*. The proposal is the only one of the five that names the cost of relaxing `--no-ff` directly, and it justifies that cost on the grounds that worker-driven mid-stack edits and conflict recovery are the entire reason `gh-stack` exists.
