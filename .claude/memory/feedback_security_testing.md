---
name: Granular auth testing over mock-all
description: Prefer real crypto signing and granular authorization tests — never use mock_all_auths()
type: feedback
---

Prefer granular auth testing over blanket mocking like `mock_all_auths()`.

**Why:** `mock_all_auths()` is an anti-pattern that masks authorization bugs. Real auth testing catches permission/signing issues that mocks hide.

**How to apply:** Use real crypto signing (Keypair, Secp256r1) in tests. Add negative tests showing unauthorized access fails. Only mock specific auth when testing something unrelated to authorization.
