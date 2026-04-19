#!/usr/bin/env bash
# pipeline-note-set.sh — set key:value lines on refs/notes/pipeline,
# replacing any existing lines for the same keys while preserving the rest.
#
# Usage:
#   scripts/pipeline-note-set.sh <commit-sha> key=value [key=value ...]
#
# Semantics:
#   1. Read the existing refs/notes/pipeline note on <commit-sha> (if any).
#   2. Drop every line matching ^(key1|key2|...):[[:space:]]* for the keys
#      being set.
#   3. Append the new `key: value` lines in the argument order given.
#   4. Force-overwrite the note via `git notes --ref=pipeline add -f -F -`.
#
# Does not push refs/notes/pipeline; the caller decides when to push.
# Must be run from inside the repo working tree so `git notes` finds the ref.
#
# Exit codes:
#   0 — success
#   1 — bad usage (missing sha, missing kv args, malformed key=value)

set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <commit-sha> key=value [key=value ...]" >&2
  exit 1
fi

sha="$1"
shift

keys=()
pairs=()
for arg in "$@"; do
  if [[ "$arg" != *=* ]]; then
    echo "error: malformed argument '$arg' (expected key=value)" >&2
    exit 1
  fi
  key="${arg%%=*}"
  value="${arg#*=}"
  if [[ -z "$key" ]]; then
    echo "error: empty key in '$arg'" >&2
    exit 1
  fi
  keys+=("$key")
  pairs+=("$key" "$value")
done

existing=$(git notes --ref=pipeline show "$sha" 2>/dev/null || true)

filter_re=""
for k in "${keys[@]}"; do
  esc=$(printf '%s' "$k" | sed 's/[][\\.^$*+?(){}|/]/\\&/g')
  if [[ -z "$filter_re" ]]; then
    filter_re="$esc"
  else
    filter_re="$filter_re|$esc"
  fi
done

filtered=""
if [[ -n "$existing" ]]; then
  filtered=$(printf '%s\n' "$existing" | grep -vE "^(${filter_re}):[[:space:]]*" || true)
fi

{
  if [[ -n "$filtered" ]]; then
    printf '%s\n' "$filtered"
  fi
  i=0
  while [[ $i -lt ${#pairs[@]} ]]; do
    printf '%s: %s\n' "${pairs[$i]}" "${pairs[$((i+1))]}"
    i=$((i+2))
  done
} | git notes --ref=pipeline add -f -F - "$sha"
