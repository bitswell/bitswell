#!/usr/bin/env bash
# project-spawn.sh — Scaffold a new bitswell project.
#
# Creates:
#   - projects/<slug>.yaml              (the manifest, committed)
#   - .loom/projects/<slug>/.gitkeep    (worktree root, so Shuttle can dispatch)
#
# Optionally links agent definitions from an actors source (another repo,
# a submodule, or the current tree's .claude/agents/) so the project's
# worktrees resolve the same roster that its manifest declares. The link
# is only created if --actors-from is passed explicitly — by default the
# tree is left alone and Claude Code inherits .claude/agents/ from the
# primary worktree as usual.
#
# Usage:
#   scripts/project-spawn.sh <slug> [options]
#
# Required:
#   <slug>                         Short kebab-case identifier. Becomes
#                                  projects/<slug>.yaml and the worktree
#                                  prefix .loom/projects/<slug>/.
#
# Options:
#   --name <display-name>          Human-readable name. Defaults to slug
#                                  title-cased.
#   --description <prose>          Free-form description written into the
#                                  manifest. Defaults to a stub the author
#                                  is expected to edit.
#   --repo <path>                  Repo path (relative to the tree root,
#                                  e.g. repos/bitswell/loom-tools). Repeat
#                                  for multiple. Defaults to none
#                                  (greenfield project).
#   --agents <csv>                 Comma-separated agent slugs authorized
#                                  on this project. Defaults to the full
#                                  standard roster (matches bitswell-core).
#   --actors-from <path>           Path to a tree whose .claude/agents/
#                                  directory should be symlinked into the
#                                  spawned project's worktree root. Use to
#                                  point at a future actors submodule.
#                                  If omitted, no link is created.
#   --github-project <url>         Project (board) URL to record. Null by
#                                  default; fill in after creating the
#                                  board.
#   --teams <csv>                  Comma-separated team names for
#                                  team-of-teams projects. When provided,
#                                  the manifest gains a `teams:` list
#                                  alongside `agents:`. Each team is
#                                  recorded with an empty agent sub-roster
#                                  for the author to fill in.
#   --force                        Overwrite an existing manifest.
#   --dry-run                      Print what would happen; touch nothing.
#   -h | --help                    This message.
#
# Examples:
#   # Minimal greenfield:
#   scripts/project-spawn.sh kiln --name Kiln \
#     --description "Long-running batch training project."
#
#   # With existing submodules + a GitHub Project:
#   scripts/project-spawn.sh forge --name Forge \
#     --repo repos/bitswell/loom-tools \
#     --repo repos/bitswell/memctl \
#     --github-project https://github.com/orgs/bitswell/projects/3
#
#   # Team-of-teams scaffold:
#   scripts/project-spawn.sh atlas --name Atlas \
#     --teams "runtime,workers,observability"
#
# After spawn, the author is expected to:
#   1. Edit the generated manifest (description, agents roster if non-default).
#   2. Commit the manifest + .loom/projects/<slug>/.gitkeep on a PR branch.
#   3. (Later) Create the GitHub Project board and fill in github_project.

set -euo pipefail

DEFAULT_AGENTS="bitswell,shuttle,bitsweller,vesper,ratchet,moss,drift,sable,thorn,glitch,bitswelt"

die() {
  echo "project-spawn: $*" >&2
  exit 1
}

usage() {
  sed -n '/^# project-spawn.sh/,/^$/p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

slug=""
name=""
description=""
repos=()
agents_csv=""
actors_from=""
github_project=""
teams_csv=""
force=0
dry_run=0

while (( $# > 0 )); do
  case "$1" in
    -h|--help)        usage 0 ;;
    --name)           name="${2:?--name requires an argument}"; shift 2 ;;
    --description)    description="${2:?--description requires an argument}"; shift 2 ;;
    --repo)           repos+=("${2:?--repo requires an argument}"); shift 2 ;;
    --agents)         agents_csv="${2:?--agents requires an argument}"; shift 2 ;;
    --actors-from)    actors_from="${2:?--actors-from requires an argument}"; shift 2 ;;
    --github-project) github_project="${2:?--github-project requires an argument}"; shift 2 ;;
    --teams)          teams_csv="${2:?--teams requires an argument}"; shift 2 ;;
    --force)          force=1; shift ;;
    --dry-run)        dry_run=1; shift ;;
    --)               shift; break ;;
    -*)               die "unknown flag: $1 (try --help)" ;;
    *)
      if [[ -z "$slug" ]]; then
        slug="$1"
        shift
      else
        die "unexpected positional argument: $1"
      fi
      ;;
  esac
done

[[ -n "$slug" ]] || die "missing <slug>. Run with --help."

# slug must be kebab-case: lowercase letters, digits, single dashes.
if ! [[ "$slug" =~ ^[a-z][a-z0-9]*(-[a-z0-9]+)*$ ]]; then
  die "slug '$slug' is not valid kebab-case (lowercase letters/digits, single dashes, starting with a letter)."
fi

repo_top=$(git rev-parse --show-toplevel 2>/dev/null) \
  || die "not inside a git worktree."
cd "$repo_top"

manifest_path="projects/${slug}.yaml"
worktree_dir=".loom/projects/${slug}"
gitkeep_path="${worktree_dir}/.gitkeep"

if [[ -e "$manifest_path" && $force -eq 0 ]]; then
  die "manifest $manifest_path already exists. Use --force to overwrite."
fi

if [[ -n "$actors_from" ]]; then
  # Resolve relative to repo root for a stable symlink target.
  if [[ ! -d "$actors_from/.claude/agents" ]]; then
    die "--actors-from '$actors_from' has no .claude/agents/ directory."
  fi
fi

# Derive defaults.
if [[ -z "$name" ]]; then
  # Title-case the slug: "foo-bar-baz" -> "Foo Bar Baz".
  name=$(echo "$slug" | awk 'BEGIN{RS="-"; ORS=" "} {print toupper(substr($0,1,1)) substr($0,2)}' | sed 's/ $//')
fi

if [[ -z "$description" ]]; then
  description="TODO: describe this project."
fi

agents_csv="${agents_csv:-$DEFAULT_AGENTS}"

# Split CSVs into bash arrays (IFS-safe, trims surrounding spaces).
IFS=',' read -r -a agents_list <<<"$agents_csv"
teams_list=()
if [[ -n "$teams_csv" ]]; then
  IFS=',' read -r -a teams_list <<<"$teams_csv"
fi

format_description() {
  local prose="$1"
  # YAML block-literal with two-space indent. Preserves newlines, keeps the
  # author's wording intact.
  echo "  $prose" | sed 's/$//'
}

render_manifest() {
  {
    echo "slug: ${slug}"
    echo "name: ${name}"
    echo "description: |"
    echo "  ${description}"
    if [[ -n "$github_project" ]]; then
      echo "github_project: ${github_project}"
    else
      echo "github_project: null  # TODO: link when a board is created"
    fi
    if (( ${#repos[@]} == 0 )); then
      echo "repos: []  # Greenfield — add submodule paths as the project develops"
    else
      echo "repos:"
      for r in "${repos[@]}"; do
        echo "  - ${r}"
      done
    fi
    echo "agents:"
    for a in "${agents_list[@]}"; do
      a_trimmed="${a## }"; a_trimmed="${a_trimmed%% }"
      [[ -n "$a_trimmed" ]] && echo "  - ${a_trimmed}"
    done
    if (( ${#teams_list[@]} > 0 )); then
      echo "teams:"
      for t in "${teams_list[@]}"; do
        t_trimmed="${t## }"; t_trimmed="${t_trimmed%% }"
        [[ -z "$t_trimmed" ]] && continue
        echo "  - name: ${t_trimmed}"
        echo "    agents: []  # TODO: roster for team '${t_trimmed}'"
      done
    fi
  }
}

if (( dry_run )); then
  echo "[dry-run] would write ${manifest_path}:"
  echo "---8<---"
  render_manifest
  echo "--->8---"
  echo "[dry-run] would create ${gitkeep_path}"
  if [[ -n "$actors_from" ]]; then
    echo "[dry-run] would symlink ${worktree_dir}/.claude/agents -> ${actors_from}/.claude/agents"
  fi
  exit 0
fi

mkdir -p "$(dirname "$manifest_path")"
mkdir -p "$worktree_dir"

render_manifest >"$manifest_path"

if [[ ! -e "$gitkeep_path" ]]; then
  touch "$gitkeep_path"
fi

if [[ -n "$actors_from" ]]; then
  link_target="${worktree_dir}/.claude/agents"
  mkdir -p "$(dirname "$link_target")"
  if [[ -L "$link_target" || -e "$link_target" ]]; then
    rm -rf "$link_target"
  fi
  # Use a path relative to the link's directory for portability.
  rel=$(python3 -c "import os,sys; print(os.path.relpath(sys.argv[1], sys.argv[2]))" \
    "${repo_top}/${actors_from}/.claude/agents" "${repo_top}/$(dirname "$link_target")")
  ln -s "$rel" "$link_target"
fi

cat <<EOF
project-spawn: created '${slug}'
  manifest:  ${manifest_path}
  worktree:  ${worktree_dir}/
EOF

if [[ -n "$actors_from" ]]; then
  echo "  actors:    ${worktree_dir}/.claude/agents -> ${actors_from}/.claude/agents (symlink)"
fi

cat <<'EOF'

Next steps:
  1. Review and edit the manifest (description, roster, repos).
  2. Create a worktree under .loom/projects/<slug>/<role>/<slug> from main.
  3. Commit the manifest on a PR branch; open a PR against main.
  4. Create the GitHub Project board and fill in github_project.
EOF
