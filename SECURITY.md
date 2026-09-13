# Security policy

TCCL's compiler and virtual machine are part of The Coin's consensus rules. A bug can let someone steal funds, split the network or stall block validation, so please report it privately.

## Reporting

- Do **not** open a public issue, pull request or discussion for a vulnerability.
- Use GitHub's private vulnerability reporting for this repository (Security → Report a vulnerability), or contact the maintainer through https://the-coin.cloud.
- Include: affected version or commit, language version, a minimal contract or input, expected and actual behaviour, and impact.

We aim to acknowledge reports within 72 hours and to agree on a disclosure date. Fixes to an active language version are coordinated with node operators before publication.

## Scope

In scope: the compiler (both language versions), the VM and fuel schedule, the standard modules, program and value decoding, the simulator when used by the node or wallet, the WebAssembly engine, installers and release artifacts.

Out of scope: bugs in individual user contracts (report them to their authors), the playground's simulated balances, and denial of service against the website.

## Known issues

- Language version 1 under-prices copies of values with many allocations (1 400–2 100 ns per fuel in adversarial contracts). Fixed in language version 2; mitigation for version 1 requires a network activation. Details: `docs/en/security.md#v1-findings` and `implant-the-coin-language.md` §6.
- Windows binaries are not code-signed yet; verify downloads with `SHA256SUMS`.
