#!/usr/bin/env bash
# protect-branch.sh — Enable branch protection rules via GitHub API.
#
# Usage:
#   protect-branch.sh [owner/repo] [branch]
#
# Defaults:
#   repo:   detected from git remote
#   branch: main
#
# Requires: gh (authenticated)

set -euo pipefail

REPO="${1:-}"
BRANCH="${2:-main}"

if [[ -z "$REPO" ]]; then
  REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null) || {
    echo "Error: could not detect repo. Pass owner/repo as first argument." >&2
    exit 1
  }
fi

echo "Protecting $REPO branch: $BRANCH"

gh api "repos/$REPO/branches/$BRANCH/protection" \
  --method PUT \
  --input <(cat <<JSON
{
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false
  },
  "required_status_checks": null,
  "enforce_admins": false,
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": false,
  "lock_branch": false,
  "allow_fork_syncing": true
}
JSON
) --jq '{
  url: .url,
  enforce_admins: .enforce_admins.enabled,
  required_pull_request_reviews: (.required_pull_request_reviews | { required_approving_review_count, dismiss_stale_reviews }),
  allow_force_pushes: .allow_force_pushes.enabled,
  allow_deletions: .allow_deletions.enabled
}' && echo "Done. Branch '$BRANCH' is now protected."
