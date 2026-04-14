# Team 1 — Review (Drift)

**Verdict: APPROVED WITH EDITS**

---

## Drift take (the one sentence)

This proposal is load-bearing on a git fact it never checks: that the workspace merge commit M_n minus M_(n-1) equals what the worker actually wrote — and that is only true when the workspace was identical to `main` at the start of the epic, which the proposal never requires, never asserts, and never detects.

Everything else follows from that. The angle is sound. The invariants are preserved. The escape hatch is real. But the projection itself — the single mechanism the whole proposal rests on — quietly depends on a property of the workspace that nothing in LOOM guarantees today.

---

## 1. Coverage audit

All seven required RFP sections are present and substantive:

| RFP §  | Proposal §           | Status          |
| ------ | -------------------- | --------------- |
| 1 Angle | §1 + §2 (thesis)    | Present. Clear. |
| 2 What changes | §3 (8 subs)  | Present. Exhaustive. |
| 3 Branch naming & scope | §4 (5 subs) | Present. Answers the `Scope:` question directly. |
| 4 Merge vs rebase | §5 (6 subs) | Present. This is the strongest section. |
| 5 Worker authority | §6 (4 subs) | Present. Short but decisive. |
| 6 End-to-end example | §7 (8 subs) | Present. Concrete to the commit level. |
| 7 Risks & rejected alternatives | §8 (6 subs, 5 rejections) | Present. Exceeds RFP minimum of "at least two rejected alternatives." |

No sections missing. No sections thin. Coverage is the strongest thing in this proposal after the angle itself.

---

## 2. Angle fidelity audit

**Angle constraint:** "read-only presentation projection" — `gh-stack` as output artifact, not integration mechanism. Any participation in the real merge path is disqualifying.

The proposal holds the line rigorously across every section:

- §3.1 declares the tool `orchestrator`-only and thin.
- §3.2 lists every existing `loom-tools` tool as unchanged.
- §4.1 is explicit: *"nothing in `stack/*` is ever the source of truth for anything."*
- §5.2 confines force-push to the mirror namespace and states the orchestrator **does not merge** the stack PR.
- §5.4 enumerates what the proposal does *not* do (§5.4 is almost a checklist against the failure mode the angle forbids).
- §6.1 holds workers entirely out of `gh stack` commands.
- §7.7 integrates via the existing `pr-create` path on `loom/*`, not via the stack.

The one place where the proposal *could* drift is §7.6 "review feedback," where the orchestrator runs `gh stack rebase --upstack` to update upper layers after a re-dispatch. Rebase on `stack/*` is fine under the angle (mirrors are disposable), but the paragraph is loose: it does not re-state that the result is not being fed back into `loom/*`. A reader skimming could mistake the upstack rebase for a merge-path mutation. **Not a drift — a clarity gap.** Suggested edit in §8 below.

**Angle fidelity: PASS.** The proposal does not let `gh-stack` participate in the real merge path anywhere.

---

## 3. Sharp-edge audit

Issue #74 lists five sharp edges. Each is either named and answered, or demoted into a reusable primitive by the angle.

| # | Sharp edge | Where named | Answer quality |
| - | ---------- | ----------- | -------------- |
| 1 | `loom-tools` already supports custom bases | §2.1 "reusable primitives" | **Demoted to primitive.** Proposal reuses existing `pr-create` base arg for the final per-`loom/*` PR. No hand-wave. |
| 2 | Dependency DAG exists | §2.1, §3.2 (dag-check), §7.2 (phase 1 trace) | **Demoted to primitive.** `integrationOrder` is the exact input `stack-project` consumes. Verified against `dag-check.ts:23` — the field name is real. |
| 3 | Branch naming conflict | §4.1 "two namespaces, one rule" | **Answered.** Separate namespaces (`loom/*` vs `stack/*`), ownership rules in §4.2, lifecycle in §4.4. No contest. |
| 4 | Merge vs rebase | §5 (all of it) | **Answered — and this is the best-argued section.** Mechanism-per-namespace table in §5.2, explicit enumeration of what the proposal does not do in §5.4, idempotence argument in §5.5, freshness-check escape hatch in §5.6. |
| 5 | Worker-vs-orchestrator authority | §6.1, §6.2 | **Answered.** Zero `gh stack` on the worker side; §6.2 maps every `protocol.md` §6.1 boundary and shows "unchanged" on each row. |

**No hand-waves on any of the five.** The proposal does the thing the RFP is begging for: names the sharp edges, commits to an answer, and lives with the consequences.

What it *does* hand-wave — and this is the important one — is a sharp edge the RFP did not explicitly list but that the proposal's projection mechanism reveals: **"what does M_n minus M_(n-1) actually contain when the workspace is not a fresh clone of `main`?"** See the feasibility audit.

---

## 4. Feasibility audit

The proposal self-describes as "one new tool (~100 lines), one new recipe, one-line dispatch filter." That estimate is honest for the happy path. It is an underestimate for two specific cases the proposal does not fully engage with.

### 4.1 Honest parts

- The `stack-project.ts` shape in §3.1 is a faithful mirror of `pr-create.ts`. I verified the real file: `roles: ['orchestrator']`, `exec()` pattern, zod schemas — the proposal's code sketch is structurally accurate.
- `dag-check.ts` really does return `integrationOrder` (verified at `src/tools/dag-check.ts:23`).
- `pr-retarget.ts` really does use the `orchestrator` role (verified at line 25).
- The proposal explicitly marks five files as "no changes" and that list holds up against the actual tool directory.
- Reversibility claim is real: delete `stack-project.ts`, drop the recipe, LOOM is exactly where it was.

### 4.2 Underestimate #1: projection content is not as clean as §7.5 suggests

This is the most important thing in this review.

§7.5 writes mirror branches by force-branching to workspace first-parent merge commits M1, M2, M3. It then claims each layer's PR shows the per-layer diff because `gh stack` sets each layer's base to the layer below. Risk 3 in §8.1 acknowledges a "subtlety" here but mitigates it with `git rerere` and a fallback to per-`loom/*` PRs.

The subtlety is deeper than §8.1 admits:

- M_n is a merge commit whose tree is "workspace-after-M_(n-1)" merged with "loom/agent-n". If the workspace has **any** prior history not in `main`, then `main..M1` shows `main..workspace + loom/agent-1` — which includes the prior history. PR #201's diff is then polluted with unrelated epic work.
- `gh stack init --adopt` rebases adopted branches onto their predecessor in the stack. Rebasing a merge commit onto `main` either flattens (losing the merge structure) or fails (if `-Xours`/`-Xtheirs` isn't used). The proposal does not specify rebase strategy.
- Per the SKILL.md reference, `gh stack init --adopt` **"rejects if any is already in a stack or has an existing PR"** (line 433). §5.5 claims idempotence, but strict re-adoption requires an explicit unstack-then-init cycle. `gh stack submit --auto` updating in place is fine; re-running `init --adopt` on branches that already belong to a stack is not. The freshness-check + re-project path in §5.6 and §7.6 conflates these.

None of these are fatal. All of them are implementation work the proposal's "~100 lines" budget does not cover. A realistic estimate:

- **stack-project.ts happy path:** 100–150 lines, as claimed.
- **Freshness check + git-notes tracking:** another 50–80 lines.
- **Unstack-before-reproject dance:** another 30–50 lines, plus careful error handling for the case where a mirror PR has been closed externally.
- **Workspace-not-equal-to-main handling:** unspecified. This is the part that could eat a week if done wrong.

A more honest total: 200–300 lines + one precondition check that `workspace_base == main` at projection time (or a cherry-pick-based mirror construction that does not depend on merge commit trees).

### 4.3 Underestimate #2: `loom-dispatch --scan`

§3.6 and §4.5 add "one line" to `loom-dispatch --scan`. I could not find a `--scan` subcommand in `loom-tools/src/tools/`. The actual tools are `dispatch.ts` and `dispatch-check.ts`; neither filters branches by `loom/*` prefix anywhere I could grep for. The proposal is either referring to functionality that does not exist yet, or to a different surface (the `loom` plugin skill, perhaps). **This is a minor factual ambiguity**, not a showstopper, but it means the "one-line guard" is either zero lines (if the functionality doesn't exist) or an unknown number of lines (if it has to be introduced). Documented in §8 below.

### 4.4 What is sized honestly

- The "no changes to X" list is genuinely zero cost.
- The freshness check as a concept is tractable.
- The end-to-end tool-call count in §7.8 (10 ordered steps) is realistic.
- The reversibility argument is rock-solid.

**Feasibility verdict:** the happy path is sized correctly; the two underestimates above mean the real implementation is closer to a week than a day, but the proposal does not claim a day — it claims "narrow orchestrator recipe plus one new tool," which is still true in spirit.

---

## 5. Strongest arguments

1. **The demotion move.** §2.1 is the one move that makes the entire proposal possible: three of five sharp edges stop being sharp the moment `gh-stack` is denied integration authority. That is a genuine architectural insight, not a rhetorical dodge. It is the move a reviewer would wish every RFP response made.
2. **The "what does not change" list.** §3 is structured as an audit of untouched surface area. Every paragraph that says "no changes" is a paragraph of risk that did not get added to LOOM. This is rare and valuable and exactly the right shape for a proposal that claims to preserve invariants.
3. **The end-to-end example is traced to the commit.** §7 walks from the three ASSIGNED commits through merge commits M1/M2/M3, through projection, through a realistic review-feedback loop, through teardown. A winning-round evaluator can verify every step against real primitives. This is what #74's review criteria explicitly reward ("a concrete trace is worth more than abstract architecture").

---

## 6. Weakest arguments

1. **The projection mechanism depends on a property of the workspace no one has declared.** See §4.2 above. If the workspace is not a fresh clone of `main` at epic start, §7.5's "force-branch to M_n" produces polluted layer diffs. Nothing in the proposal detects or prevents this. A winner-selection reviewer who asks "what happens if the workspace already has unrelated work?" gets no answer. **This is the single issue most likely to sink the proposal.**
2. **Idempotence is asserted, but `gh stack init --adopt` semantics push against it.** §5.5 and §8.1 Risk 2 hand-wave the re-projection path. A rigorous implementation needs `gh stack unstack` → recreate branches → `gh stack init --adopt` → `gh stack submit --auto --draft` on every upstream change, with explicit handling of externally-closed mirror PRs. The proposal says "re-run the recipe" and moves on.
3. **"Only a cognitive tax" undersells the reviewer cost.** §2.3 and Risk 1 claim three layers of defense (title prefix, body template, branch protection). In practice a reviewer whose daily dashboard lists 40 PRs does not read PR bodies; they click titles. A mirror PR that looks like a normal PR but whose "Approve" button is semantically meaningless is the kind of UX that gets debugged in a postmortem six months later. The proposal owns this cost at the level of "reviewers must understand" and does not engage with what happens when they don't.

---

## 7. Suggested edits to proposal.md

### Edits made

1. **§3.2, file name correction.** Changed `commit-validate.ts` → `trailer-validate.ts`. The tool directory contains `trailer-validate.ts`, not `commit-validate.ts`. Factual error; amended.

### Edits recommended but not made (writer's voice, not fact-check territory)

2. **§4.5 / §3.6, `loom-dispatch --scan`.** Clarify which tool this refers to. I could not find a `--scan` subcommand in `loom-tools/src/tools/` — the actual dispatch tools are `dispatch.ts` and `dispatch-check.ts`. Either rename this to the real tool, or state that the exclusion lives in the `loom` plugin skill's dispatch recipe rather than in `loom-tools`. Not amended: ambiguous enough that I could be missing context the writer has, and the fix could go several ways.

3. **§5 or §7.5, workspace-base invariant.** The projection mechanism assumes the workspace is equivalent to `main` at epic start, or at minimum that M_1's content diffed against `main` gives "only what agent 1 wrote." This should be stated explicitly as a **precondition** with a detection strategy: either `stack-project` fails cleanly if `git merge-base workspace main` is not `main`, or it constructs mirror branches by cherry-picking from `loom/*` directly rather than force-branching to workspace merges. Not amended: this is substantive design, not a fact-check. It belongs to the writer and/or the winner-selection round.

4. **§5.5 idempotence + `gh stack init --adopt` re-run.** Acknowledge that idempotent re-projection requires `gh stack unstack` before re-`init --adopt`, per the SKILL reference's rule that `init --adopt` "rejects if any branch is already in a stack or has an existing PR." Not amended: the fix involves small code-path additions that change the tool's shape; writer's call.

5. **§7.6 upstack-rebase clarity.** Add one sentence re-stating that the `gh stack rebase --upstack` call mutates only `stack/*`, never `loom/*`. The paragraph is technically correct but skimmable in a way that could mislead. Not amended: this is a style preference and amending it would cross into the writer's voice.

---

One question to leave the writer with:

If the workspace already contains prior epic merges when `stack-project` runs, does PR #201 show the current epic's layer 1 — or does it show the current epic's layer 1 plus everything else that has landed in the workspace since `main` last advanced?

— Reviewed by Drift
