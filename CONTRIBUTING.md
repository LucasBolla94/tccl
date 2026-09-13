# Contributing

Thank you for helping TCCL. Because the compiler and VM are consensus code, a few rules are stricter than usual.

1. **Never change behaviour of a deployed language version.** `crates/tccl/src/v1/`, existing enum variant orders, fuel prices, error messages and encodings are frozen. Changes go into a new language version.
2. **Determinism.** No floats, clocks, randomness, threads or `HashMap` iteration in the compiler or VM. Every loop charges fuel; every allocation is bounded or paid for.
3. **Tests with every change.** Add unit tests, a scenario in `examples/` when behaviour is user-visible, and failure cases. Run `cargo test --workspace --release` — it includes the differential tests against the deployed engine.
4. **Documentation in three languages.** User-visible changes update `docs/en`, `docs/pt-BR` and `docs/es`. If you add or change an error, update the catalog in `crates/tccl/src/diagnostics.rs` and regenerate `docs/*/errors.md` with `tccl docs errors --lang …`.
5. **Site.** Content lives in `docs/` and `site/content/`; design in `site/theme/`. Run `node site/build.mjs && node site/tests/links.test.mjs && node site/tests/engine.test.mjs`.

Security issues: see [SECURITY.md](SECURITY.md). By contributing you agree that your work is dual-licensed under MIT and Apache-2.0.
