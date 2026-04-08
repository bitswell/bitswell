---
name: GitHub SSH setup for bitswell account
description: SSH key and host alias configured for pushing to bitswell/bitswell repo
type: reference
---

The `bitswell` GitHub account uses a dedicated SSH key:
- Key: `~/.ssh/id_ed25519_bitswell`
- SSH config Host alias: `github-bitswell` (in `~/.ssh/config`)
- Remote URL: `git@github-bitswell:bitswell/bitswell.git`
- `GITHUB_TOKEN` is set in `.claude/settings.local.json` for `gh` CLI auth as `bitswell`

Willem's personal SSH key (`id_ed25519`) authenticates as `willemneal`, which does NOT have push access to the bitswell repo.
