# Team 4 — OUTLINE

## Angle statement

**Worker-side stacking with delegated authority: for stack-epic assignments, LOOM explicitly delegates `gh stack` authority to workers, who init, add, rebase, and submit from inside their own worktrees, and the trust model shifts from "orchestrator owns all GitHub writes" to "orchestrator owns the audit contract, workers own the stack mechanics."**

(Collision-detection key: *worker-authority stacking*, *delegated-stack stacking*, or *worker-driven stacking* — all three refer to the same angle. Explicitly NOT "post-hoc projection" — that is team 1's reassigned angle.)

## Thesis

Every other plausible integration leaves the worker out of the `gh stack` call path and pays a price for it: post-hoc projection (team 1) cannot react to live rebases, MCP wrappers force every stack op through an orchestrator round-trip, and convention-only recipes either give up on stacks mid-work or require the orchestrator to pretend it is the branch author. All of them fight the grain of `gh-stack`, which was designed on the assumption that *the person making the commits also drives the stack*. This proposal aligns with that grain: for stack-epic assignments, the worker is the branch author and the stack driver, full stop. The bet is that **proximity of data beats protocol purity**. Stack rebases, conflict recoveries, and mid-stack edits all need the exact working-tree state the worker already has — forcing those ops through an orchestrator means serializing on a process that has no working tree, no rerere cache, and no memory of the last conflict. Worker-side stacking reduces orchestrator round-trips from O(branches × rebases) to O(1) per epic, eliminates the "who force-pushed my stack" race, and puts ownership where the information lives. The cost is real: LOOM's scope and audit invariants must be *deliberately* relaxed for stack-epic workers, and a new trust tier must be introduced. This proposal is the only one that names that cost in the angle statement itself instead of hiding it behind projection tricks.

## Section headers for `proposal.md`

The writer must fill in all seven RFP-required sections in the order below. Each header lists 2–5 bullet claims the writer must argue. **Section 5 is the core of this angle** and must be written first-draft in full before sections 1–4 and 6–7.

### 1. Angle statement

- Restate the one-sentence angle verbatim: worker-side stacking with delegated authority, orchestrator owns the audit contract, workers own stack mechanics.
- Name the collision-detection key: *worker-authority stacking*. Explicitly disjoint from team 1's *post-hoc projection* and from any proposal that keeps workers on the orchestrator side of `gh stack`.
- Frame the bet up front: *proximity of data beats protocol purity*. Commit to that tradeoff in one sentence — the writer must not soften it.
- State the hard constraint the reviewer will check: any sentence in this proposal implying workers do not run `gh stack` commands is a bug, and must be rewritten.

### 2. What changes

- **`loom-tools`**: add new tool `stack-worker-init` with `roles: ['worker']` (a role value that does not yet exist — writer must propose adding it to the tool schema in `src/types/tool.ts`). This tool exposes a scoped subset of `gh stack` (init, add, rebase, submit, sync, view) to workers, running inside the worker's worktree. The existing `pr-create` and `pr-retarget` tools remain `roles: ['orchestrator']` and continue to own non-stack PR creation.
- **Role enum expansion**: the writer must cite `repos/bitswell/loom-tools/src/tools/pr-create.ts` line 27 (`roles: ['orchestrator']`) and explain that today the role enum is effectively binary. This proposal adds a third role `worker-stack-driver` (or equivalent) that is a strict superset of `worker` for stack-epic assignments only.
- **Plugin/skill surface**: extend the `loom` skill with a `stack-epic` recipe that (a) the orchestrator invokes during decomposition to mark an epic as stack-mode, and (b) the worker template loads additional instructions from when the assignment carries a `Stack-Epic: true` trailer.
- **Worker template**: edit `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/worker-template.md` to add a Section 10 — "Stack-epic workers" — that overrides Section 9 (scope enforcement) with the relaxed rules defined in section 5 of this proposal.
- **Schemas**: add `Stack-Epic: <true|false>` and `Stack-Position: <integer>` trailers to `schemas.md §3.3` (assignment trailers). These are REQUIRED on ASSIGNED commits for stack-epic branches, FORBIDDEN on non-stack branches. Also add `Stack-Op: <init|add|rebase|submit|sync>` to §3.2 (state trailers) so worker stack ops are auditable in commit history.
- **File paths to touch** (writer lists them concretely): new `repos/bitswell/loom-tools/src/tools/stack-worker-init.ts`, modifications to `repos/bitswell/loom-tools/src/types/tool.ts` (role enum), registration in `repos/bitswell/loom-tools/src/tools/index.ts`, amendment to `worker-template.md`, amendment to `schemas.md §3.3`, and a new recipe file under the loom skill directory.

### 3. Branch naming and scope

- LOOM branches in a stack epic use a compound convention: `loom/<lead-agent>-<epic-slug>/<layer-slug>`. The lead agent is assigned the entire stack; each layer has its own slug. `gh-stack`'s `-p` prefix is set to `loom/<lead-agent>-<epic-slug>` and subsequent `gh stack add` calls pass only the layer suffix, matching the SKILL's prefix-handling rules.
- One worker owns the whole stack (not one worker per layer). This is a deliberate departure from LOOM's "one agent per branch" rule and the writer must call it out. Rationale: `gh stack` ops require sequential access to the worktree and rerere cache; parallel workers on a shared stack would race on the stack file and trip exit code 8 (stack locked).
- **`Scope:` enforcement across a stack is relaxed**: the assignment's `Scope:` trailer is a *union* across all intended layers, and the orchestrator verifies at integration time that every commit on every stack branch touches only files in the union scope. A per-branch scope trailer (`Scope-Layer: <layer>:<paths>`) is optional and advisory.
- **Scope enforcement when workers touch branches beyond their own**: because the stack is owned by a single worker, "branches beyond their own" means branches the worker created via `gh stack add`. These are all in the worker's worktree, all under the same `loom/<lead-agent>-<epic-slug>/*` namespace, and all covered by the union scope. The orchestrator runs scope verification against the entire branch set at integration, not per-branch. The writer must show the verification pseudocode.
- Edge case the writer must address: what happens if a stack-epic worker calls `gh stack rebase` on a layer whose scope it does not own. Answer: the rebase still happens (git allows it), but integration rejects any commit whose diff touches out-of-scope paths, so the worker gains nothing by cheating.

### 4. Merge vs rebase

- The writer must open section 4 with the direct statement: "This proposal breaks LOOM's `--no-ff` merge invariant for stack-epic branches, and here is why the trade is worth it."
- `gh-stack` rebases and force-pushes by design. Workers run `gh stack rebase` and `gh stack submit` from inside the worktree, which means worker commits are rewritten and force-pushed over their own history before integration. LOOM's integration step cannot `--no-ff` merge branches whose history was rewritten after the ASSIGNED commit — the ASSIGNED commit is no longer a parent.
- **Relaxation**: for stack-epic branches, LOOM's integration step (`protocol.md §3.3`) is replaced with a *cascading fast-forward merge*. The orchestrator merges the bottom of the stack into `main` with `--ff-only`, then the next layer into `main` with `--ff-only`, and so on. No merge commits. The audit trail is preserved via worker commit trailers (every stack op carries `Stack-Op:` and `Agent-Id`), not via `--no-ff` merge commits.
- **Audit trail preservation**: the writer must show concretely how `git log --first-parent main` still reconstructs the full sequence of stack ops after integration. Every worker commit carries `Agent-Id`, `Session-Id`, and `Stack-Op`, so the audit trail is in the commit trailers rather than in merge-commit structure. The trailer-based audit is *stronger* than `--no-ff` structure because it survives rebases, which `--no-ff` does not.
- **Relaxed invariant (named explicitly)**: LOOM's rule "integration uses `--no-ff` for audit trail" is relaxed to "integration uses `--ff-only` for stack-epic branches; audit trail lives in trailers, not merge commit structure." The writer must cite `protocol.md §3.3` and explicitly identify the clause being relaxed.
- **Compare-and-contrast paragraph**: show that team 1's post-hoc projection preserves `--no-ff` by moving stacking *after* integration, paying the cost that workers cannot react to conflicts during work. This proposal makes the opposite bet: take the audit-structure hit to buy worker-driven conflict recovery. The writer must name this tradeoff in plain language.

### 5. Worker authority — THE CORE SECTION

This is the load-bearing section of this proposal. The writer must fill it out in full before any other section. Every other section is downstream of the trust-boundary rethink here.

- **What workers gain the right to do** (enumerate exhaustively): `gh stack init`, `gh stack add`, `gh stack rebase` (all variants including `--continue` and `--abort`), `gh stack submit --auto --draft`, `gh stack sync`, `gh stack view --json`, `gh stack push`, `gh stack checkout` by branch name, `gh stack up/down/top/bottom`, `gh stack unstack`. Workers do NOT gain the right to run `gh pr create`, `gh pr merge`, `gh pr edit`, or any non-stack `gh` command. The grant is scoped to the `gh stack` subcommand tree.
- **Which LOOM invariants are relaxed** (the writer MUST list these by name, quote the relevant section, and justify each):
  1. **`protocol.md §6.1` "Workspace write — Only the orchestrator writes to the workspace. Agents MUST NOT."** — *Relaxed* for stack-epic workers: workers still do not write to the primary workspace, but they now force-push to remote refs under `loom/<lead-agent>-<epic-slug>/*`, which the orchestrator previously considered its exclusive write surface. Justification: force-pushes during stack rebase are mechanically required by `gh-stack` and cannot be serialized through the orchestrator without losing rerere state.
  2. **`protocol.md §6.1` "Agent scope — An agent may modify only files matching its `Scope` trailer."** — *Relaxed to a union*: scope is enforced across the stack's union rather than per-branch. Section 3 of this proposal specifies the verification algorithm.
  3. **`protocol.md §3.3` `integrate()` — the `--no-ff` merge step.** — *Replaced* with cascading `--ff-only` for stack-epic branches, as detailed in section 4.
  4. **`pr-create.ts` `roles: ['orchestrator']` precedent.** — *Partially relaxed*: a new role tier is introduced (`worker-stack-driver`) that grants access to the `gh stack` subcommand tree only. The orchestrator-only guard remains in force for `pr-create`, `pr-retarget`, and every non-stack PR tool.
  5. **`worker-template.md` §9 "Scope Enforcement — You MUST NOT modify files outside `scope.paths_allowed`."** — *Relaxed to union scope* per relaxation 2 above.
  6. **`protocol.md §2` one-agent-per-branch rule (implicit in the state machine).** — *Relaxed*: one worker owns multiple branches (one per stack layer) under the same assignment. The ASSIGNED commit declares the stack via `Stack-Epic: true` and a list of layer slugs.
- **How scope enforcement still works when workers touch branches beyond their own**: the writer must spell out the verification algorithm. Pseudocode:
  ```
  on integrate(stack-epic-assignment):
    union_scope = assignment.scope  # already a union across layers
    for branch in stack.branches:
      for commit in branch.commits_since_assigned:
        for path in commit.files_changed:
          if path not in union_scope:
            reject integration
    verify branch topology matches Stack-Position trailers
    verify Stack-Op trailers form a valid gh-stack sequence
    cascading ff-only merge
  ```
  Scope enforcement is stricter in one sense (every commit on every branch is checked) and looser in another (checked against the union, not per-branch).
- **How the audit trail is preserved**: every worker stack op emits a commit with `Stack-Op: <op>`, `Agent-Id`, `Session-Id`, and `Heartbeat`. The commit on each layer branch is the ground truth. `git log --all loom/<lead>-<epic>/*` reconstructs the full sequence of ops. The writer must show an example `git log` extraction that recovers the stack ops in order. The trailer-based audit is strictly *more* information than `--no-ff` merge structure provides today — merge commits tell you when branches landed, but not when they were rebased, conflict-recovered, or reordered mid-work.
- **The new trust tier — `worker-stack-driver`**: introduce the concept explicitly. Workers assigned to stack-epic tasks run with elevated authority over the `gh stack` subcommand tree and over force-pushes within their epic's branch namespace. The orchestrator's ASSIGNED commit grants this by carrying `Stack-Epic: true`. Any worker receiving an assignment without that trailer remains on the old trust tier.
- **The orchestrator's retained authority**: the orchestrator remains the sole party that (a) creates ASSIGNED commits, (b) decides whether an epic is a stack epic, (c) verifies scope at integration, (d) runs the cascading `--ff-only` integration, (e) opens non-stack PRs via `pr-create`, and (f) closes/merges PRs on GitHub. Workers cannot merge their own stacks to `main`; `gh stack submit --auto --draft` only creates draft PRs.
- **Answer the question the feedback commit demanded**: *What happens to LOOM's trust model when we let workers drive stacking?* The trust model shifts from *mechanism control* (orchestrator owns every GitHub write) to *contract control* (orchestrator owns the scope contract, the integration contract, and the audit contract; workers own the mechanics within those contracts). The orchestrator stops being a bottleneck for mechanical operations and becomes a verifier of worker outputs. The writer must include this paragraph verbatim as the closing of section 5.
- **Failure modes the writer must address**:
  1. A worker force-pushes outside its stack namespace → integration scope check rejects the branch.
  2. A worker runs `gh stack submit --auto` without `--draft` → the assignment spec forbids this; integration rejects the branch if non-draft PRs exist under the epic namespace.
  3. A worker invokes `gh pr create` instead of `gh stack submit` → the MCP `stack-worker-init` tool does not expose `pr create`; a worker bypassing it via raw `gh` is caught by integration's "no PRs outside stack-worker-init sessions" check.
  4. A worker gets compacted mid-rebase → `gh stack rebase --continue` is idempotent and recoverable from commit history; worker template section 10 prescribes the recovery procedure.

### 6. End-to-end example

Trace the canonical `auth → api → frontend` three-agent epic. Because this proposal uses one worker per stack, the "three agents" become "one worker, three layers." The writer must cover every numbered step in order and include commit-message excerpts.

- **Epic decomposition**: orchestrator creates ONE ASSIGNED commit on branch `loom/ratchet-auth-stack` with body:
  ```
  task(ratchet): stack epic — auth → api → frontend

  Agent-Id: bitswell
  Session-Id: <uuid>
  Task-Status: ASSIGNED
  Assigned-To: ratchet
  Assignment: auth-stack
  Stack-Epic: true
  Stack-Position: 1:auth-middleware,2:api-endpoints,3:frontend
  Scope: src/auth/**, src/api/**, src/frontend/**, tests/**
  Dependencies: none
  Budget: 120000
  ```
  Only one branch is created at assignment time; the worker creates layer branches via `gh stack add`.
- **Worker start**: ratchet commits `chore(loom): begin auth-stack` with `Task-Status: IMPLEMENTING` and `Stack-Op: init`. First real command is `gh stack init -p loom/ratchet-auth-stack auth-middleware` (note: the prefix embeds the loom namespace so `gh stack add api-endpoints` creates `loom/ratchet-auth-stack/api-endpoints`).
- **Layer 1 — auth-middleware**: ratchet writes auth middleware code, `git commit` with `Stack-Op: commit`, `Agent-Id`, `Session-Id`, `Heartbeat`. Multiple commits allowed per layer.
- **Layer 2 — api-endpoints**: `gh stack add api-endpoints` — worker commits `chore(loom): stack add api-endpoints` with `Stack-Op: add`. Writes API code, more commits.
- **Mid-stack edit**: worker realises auth middleware needs a helper function. `gh stack down`, edit, commit, `gh stack rebase --upstack`, `gh stack top`. Each op is a commit with `Stack-Op:` trailer. Writer must show the commit graph at this point.
- **Layer 3 — frontend**: `gh stack add frontend`, write frontend code, commits.
- **Conflict recovery**: show one conflict during `gh stack rebase --upstack`, worker resolves, `git add`, `gh stack rebase --continue`. Commit trailer: `Stack-Op: rebase-continue`.
- **Submit**: `gh stack submit --auto --draft` — worker commits `feat(auth-stack): submit draft stack` with `Task-Status: COMPLETED`, `Files-Changed`, `Key-Finding: three draft PRs created`, `Stack-Op: submit`, and the PR numbers in the body. Note: this is a non-trivial relaxation — the worker is creating PRs, even if drafts.
- **Integration phase**: orchestrator runs scope verification (union scope across all three branches), verifies `Stack-Op:` sequence is valid, verifies draft-only flag, then cascading `--ff-only` merge: `git merge --ff-only loom/ratchet-auth-stack/auth-middleware` → `loom/ratchet-auth-stack/api-endpoints` → `loom/ratchet-auth-stack/frontend`. Draft PRs on GitHub are marked ready and auto-merged, or closed in favour of the direct fast-forward (writer picks one and justifies).
- **Audit recovery**: show the `git log --format='%H %s %(trailers:key=Stack-Op)' main` output that reconstructs the full worker stack-op sequence post-integration.

### 7. Risks and rejected alternatives

- **Risk 1 — worker misbehaves with elevated authority**: a stack-epic worker could in principle force-push garbage over its stack. Mitigation: integration-time scope verification against union scope, Stack-Op sequence validation, and mandatory draft-only submission. The attack surface is bounded to the worker's own stack namespace.
- **Risk 2 — audit trail depends on trailer discipline**: if a worker forgets `Stack-Op:` trailers, the audit reconstruction loses fidelity. Mitigation: `stack-worker-init` tool auto-stamps `Stack-Op:` trailers on every op; workers cannot run `gh stack` except through the tool.
- **Risk 3 — one-worker-per-stack ceiling on parallelism**: because the stack is serial by construction, a stack epic cannot be split across parallel workers. Mitigation: this is a property of `gh-stack` itself (strictly linear stacks), not of this proposal; parallel epics use separate stacks.
- **Risk 4 — relaxing `--no-ff` weakens the audit invariant for stack-epic branches**: named explicitly, justified by the trailer-based audit being strictly more informative. The writer must call this a *deliberate trade*, not a concession.
- **Rejected alternative A — post-hoc projection (team 1's angle)**: orchestrator adopts branches after integration. Rejected because it requires workers to coordinate mid-work without stack tools, losing the exact benefit this proposal buys: worker-driven conflict recovery.
- **Rejected alternative B — orchestrator-only `gh stack` via MCP wrapper**: every stack op is a round-trip through the orchestrator. Rejected because it serializes conflict recovery through a process with no working tree and no rerere cache, and because orchestrator context compaction would lose mid-rebase state.
- **Rejected alternative C — keep orchestrator-only but pre-create all stack branches upfront**: orchestrator runs `gh stack init` before workers start, workers commit but never rebase. Rejected because it cannot handle mid-stack edits or conflict recovery — both of which require someone on the worktree with `gh stack` authority.
- **Rejected alternative D — full `gh` grant to all workers (not just stack-epic)**: simpler role model, but massively wider blast radius. Rejected because non-stack assignments have no need for `gh`, and widening the grant unnecessarily violates least-privilege.

## Key references

The writer must read these files before writing `proposal.md`. Paths are absolute unless noted.

- **Feedback commit (rejection reason)**: `git log -1 --format='%B' HEAD` on this branch — read before writing; the first-draft outline's angle was reassigned away from post-hoc projection precisely because all five teams converged on it. This outline is the replacement.
- **Epic RFP (authoritative requirements)**: `gh issue view 74 --repo bitswell/bitswell` — the seven required sections are quoted from here, and sharp edge 5 ("Worker-vs-orchestrator PR authority") is the exact question this proposal answers in the opposite direction from every other team.
- **`gh-stack` SKILL**: `/home/willem/.agents/skills/gh-stack/SKILL.md` — read the "Agent rules" section (all ops non-interactive), "Workflows → Making mid-stack changes" (the exact pattern a worker runs during a stack edit), "Workflows → Handle rebase conflicts" (the recovery procedure the worker template section 10 must reference), and exit code 8 ("Stack is locked") which is the reason one worker owns the whole stack rather than one worker per layer.
- **LOOM protocol**: `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` — §2 state machine (one-agent-per-branch rule is relaxed), §3.3 `integrate()` (the `--no-ff` merge step is replaced with cascading `--ff-only` for stack-epic branches), §6.1 security model (workspace write, agent scope, and cross-agent isolation rules are all explicitly relaxed in this proposal and must be quoted).
- **LOOM schemas**: `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md` — §3.3 assignment trailers (proposal adds `Stack-Epic` and `Stack-Position`), §3.2 state trailers (proposal adds `Stack-Op`), §4.1 required trailers per state (writer must show the extended required-trailer table for stack-epic ASSIGNED commits).
- **LOOM worker template**: `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/worker-template.md` — §9 scope enforcement (relaxed to union), §4 the work loop (workers now run `gh stack` ops here), proposed new §10 "Stack-epic workers" that the proposal must draft.
- **`pr-create.ts`**: `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts` — line 27, `roles: ['orchestrator']` is the precedent this proposal partially relaxes by introducing a new role tier. The file is the template for the new `stack-worker-init` tool except for the `roles` value.
- **`pr-retarget.ts`**: `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts` — line ~24, same `roles: ['orchestrator']` guard remains in force; the writer must state that `pr-retarget` is NOT modified because stack bases are set by `gh stack submit`, not `pr-retarget`.
- **`dag-check.ts`**: `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts` — the topological sort still runs, but for stack epics it operates on a single assignment with layer positions rather than on multiple assignments. Writer must explain this reuse with one code snippet.
- **Tool registration**: `repos/bitswell/loom-tools/src/tools/index.ts` (writer confirms exact path) — where `stack-worker-init` must be registered, and where the role enum lives.
- **Tool types**: `repos/bitswell/loom-tools/src/types/tool.ts` (writer confirms exact path) — the `roles` field's enum definition is the file that must be amended to add the new `worker-stack-driver` role.
