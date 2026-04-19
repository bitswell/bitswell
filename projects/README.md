# projects/

A **project** is a focused slice of work across some subset of the repos in this org — with its own GitHub Project (board) and its own agent roster. One manifest per project, committed to `main`. Because manifests are ordinary tracked YAML, every scope change, board link, and roster shift is an ordinary commit — "project info in git history."

## Schema

Each `projects/<slug>.yaml` has:

- **`slug`** — short, lowercase, dash-separated. Matches the filename. Used in worktree paths and labels.
- **`name`** — human-readable display name.
- **`description`** — what the project is and why it exists. Free-form prose.
- **`github_project`** — URL of the GitHub Project (board) for this slice, or `null` if not yet created.
- **`repos`** — list of repo paths (relative to this tree, e.g. `repos/bitswell/loom-tools`) that fall inside the project's scope. May be empty for greenfield projects.
- **`agents`** — roster of agent slugs authorized to act on this project.
- **`teams`** *(optional)* — for team-of-teams projects. List of `{ name, agents }` sub-rosters. When present, `agents` remains the project-level roster and `teams` partitions it.

## Spawning a new project

Use `scripts/project-spawn.sh <slug>` to scaffold a new manifest and worktree root in one step. The script writes `projects/<slug>.yaml` and creates `.loom/projects/<slug>/.gitkeep` so Shuttle can dispatch into it.

```sh
# Minimal:
scripts/project-spawn.sh kiln --description "Long-running batch training."

# With submodules, GitHub Project, team-of-teams, or actor reuse:
scripts/project-spawn.sh forge \
  --repo repos/bitswell/loom-tools \
  --github-project https://github.com/orgs/bitswell/projects/3 \
  --teams "runtime,workers" \
  --actors-from .
```

`--dry-run` prints the manifest without touching the tree. `--help` lists every flag. `--actors-from <path>` symlinks that path's `.claude/agents/` into the spawned project's worktree root — the seam a future actors submodule slots into without reworking the tool.

## Current projects

- **`bitswell-core`** — the bitswell agent system itself: identity discovery, LOOM protocol, pipeline visibility, and supporting tools.
- **`ember`** — imminent Hetzner pilot-light orchestrator that summons ephemeral RunPod GPU workers over a private Tailscale mesh.

## Status

No agent reads these manifests yet. This is **Step 1** of the team/project abstraction — a tree-only foundation. Subsequent bitsweller issues will wire consumers (worktree path scoping, PR-as-issue GHA, task-branch model).

See `CLAUDE.md` § "Pipeline Visibility" for how projects will plug into the existing `refs/notes/pipeline` / retros / merge-trailer loop.
