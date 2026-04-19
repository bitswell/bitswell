# Ember Architecture

Ember is a pilot-light GPU orchestrator: one always-on Hetzner VPS summons ephemeral RunPod GPU workers on demand and joins them to a Tailscale mesh. The pilot is cheap and idle most of the time; the workers are expensive and short-lived. This document pins the five decisions every downstream issue will read against.

## Hetzner VPS size

**Recommendation:** CX22 (2 vCPU / 4 GB / €3.79).

The pilot runs SSH, a small HTTP control API, and a Tailscale client — none of which stress 2 vCPU / 4 GB. CX32 becomes justified only if the pilot ever starts holding worker state (queues, artifact cache); flag that as the flip point.

## RunPod GPU dispatch

**Recommendation:** On-demand pods triggered by a direct GraphQL call from the pilot, with a per-pod 2-hour runtime cap and a €20/day spend ceiling enforced pilot-side before the API call.

Spot is cheaper but eviction mid-job is a pilot-light footgun; on-demand keeps the trigger path synchronous and the failure modes few. Guardrails live on the pilot rather than in RunPod because that is the only place we fully trust.

## Tailscale topology

**Recommendation:** Direct peer mode — pilot and workers are individual tailnet nodes, no subnet router. Tag workers `tag:ember-worker` and the pilot `tag:ember-pilot`; ACL permits `tag:ember-pilot → tag:ember-worker:22` only.

Subnet routing earns its weight when bridging a LAN; for point-to-point SSH between two Tailscale nodes it is pure overhead. ACL-by-tag scopes blast radius if a worker's key ever leaks.

## Control-plane surface

**Recommendation:** A single CLI binary on the pilot — `ember summon --gpu 4090 --image …` — that blocks until the worker is reachable and prints its MagicDNS name (e.g. `ember-worker-abc.tail-scale.ts.net`) on stdout.

A CLI is the smallest surface that still composes in shell pipelines; HTTP and event-driven designs become warranted once there is more than one caller or the summon is cross-host. MagicDNS avoids baking IPs into callers.

## Secrets + auth

**Recommendation:** sops-encrypted secrets file in the pilot's repo, decrypted at boot with an age key stored on the pilot's root-owned disk; RunPod API key, Tailscale auth key, and Hetzner token all live in that file. Rotate the RunPod and Tailscale keys quarterly, Hetzner token on personnel change.

sops+age is the lightest tool that survives audit — plain env vars lose the rotation story, and Vault is infrastructure the pilot cannot justify. If the pilot ever grows from one operator to a team, revisit: OIDC-fronted Tailscale + short-lived RunPod tokens becomes the better shape.

## Future work

- ember-orchestrator repo bootstrap (pilot codebase — CLI, HTTP shim if needed, provisioning script).
- sops/age workflow: key generation, commit encrypted file, pilot boot-time decrypt hook.
- Hetzner provisioning script (Terraform or cloud-init) with CI that validates it applies cleanly.
- RunPod GraphQL client module with the per-pod runtime cap and daily spend ceiling baked in.
- Tailscale ACL definition committed alongside the orchestrator, applied via `tailscale set --advertise-tags`.

— Built by Moss
