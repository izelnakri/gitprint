# Changelog

## [0.3.7] - 2026-03-01
[`v0.3.6...v0.3.7`](https://github.com/izelnakri/gitprint/compare/v0.3.6...v0.3.7)

### Features
- Now users can do $ gitprint -u izelnakri --commits 100 — 2026-02-20 by [@izelnakri](https://github.com/izelnakri) ([`d802f94`](https://github.com/izelnakri/gitprint/commit/d802f94c3f53a357cb9e31da82c0ff78b9021977))
- Neon diff palette, rate-limit errors & user report refactor — 2026-03-01 by [@izelnakri](https://github.com/izelnakri) ([`b429f8b`](https://github.com/izelnakri/gitprint/commit/b429f8b1a41b235b5db52ca78487af59ed164c98))

## [0.3.6] - 2026-02-20
[`v0.3.5...v0.3.6`](https://github.com/izelnakri/gitprint/compare/v0.3.5...v0.3.6)

### Documentation
- Add Docker installation guide & nix-aware make release — 2026-02-19 by [@izelnakri](https://github.com/izelnakri) ([`b0486ac`](https://github.com/izelnakri/gitprint/commit/b0486ac10fbc008f181ecec2afd5b398516b000e))

### Features
- Fix FS Size display & add docker nightly builds — 2026-02-19 by [@izelnakri](https://github.com/izelnakri) ([`0f47d9b`](https://github.com/izelnakri/gitprint/commit/0f47d9b005cc1d42a604e19ae30d07a142732f8c))
- -u flag now generates user reports! — 2026-02-20 by [@izelnakri](https://github.com/izelnakri) ([`a849ab9`](https://github.com/izelnakri/gitprint/commit/a849ab98a6ad860546b4957c9e2101c957f64c62))

## [0.3.5] - 2026-02-19
[`v0.3.4...v0.3.5`](https://github.com/izelnakri/gitprint/compare/v0.3.4...v0.3.5)

### Features
- Metadata Page "Size" is now "Repo Size" or "FS Size" — 2026-02-19 by [@izelnakri](https://github.com/izelnakri) ([`ff181e4`](https://github.com/izelnakri/gitprint/commit/ff181e402793ef1fa7dfd3983da110dfd81fac7f))
- ONLY ALLOW RELEASE OF BINARY mean benchmark < $REGRESSION_TRESHOLD — 2026-02-19 by [@izelnakri](https://github.com/izelnakri) ([`31acc68`](https://github.com/izelnakri/gitprint/commit/31acc68908b1a43066b93f28486b4c636f16fed7))

## [0.3.3] - 2026-02-19
[`v0.3.2...v0.3.3`](https://github.com/izelnakri/gitprint/compare/v0.3.2...v0.3.3)

### Features
- Improve release workflow — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`e564a7e`](https://github.com/izelnakri/gitprint/commit/e564a7e11ef8f9131b7225ced78405dca6e196a6))
- Optimized release binaries, interactive release flow, CHANGELOG commit links — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`db7d557`](https://github.com/izelnakri/gitprint/commit/db7d55757eb76dfca3152481f7d597d2a1d49464))

## [0.3.2] - 2026-02-18
[`v0.3.1...v0.3.2`](https://github.com/izelnakri/gitprint/compare/v0.3.1...v0.3.2)

### Features
- Add make fix, cargo binstall support, and faster nix run — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`25eb991`](https://github.com/izelnakri/gitprint/commit/25eb99167a8254f4f37434fd058b4932ca2a339a))
- Accept remote URLs as input (git clone + generate PDF) — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`b14e2b1`](https://github.com/izelnakri/gitprint/commit/b14e2b180058963b17f034da23eaa228c55f4ec9))
- Improve Metadata first page — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`c7fc8d7`](https://github.com/izelnakri/gitprint/commit/c7fc8d7d5eff316f4b21612c88ac35c47098626b))

## [0.3.1] - 2026-02-18
[`v0.3.0...v0.3.1`](https://github.com/izelnakri/gitprint/compare/v0.3.0...v0.3.1)

### Features
- Slim down dependency tree further — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`88ccf92`](https://github.com/izelnakri/gitprint/commit/88ccf92296fe013c943b601fb478fb06ddb7d0fc))

## [0.2.0] - 2026-02-18
[`v0.1.2...v0.2.0`](https://github.com/izelnakri/gitprint/compare/v0.1.2...v0.2.0)

### Bug Fixes
- Ensure musl target stdlib is installed on cold cache; add README badges — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`6e28b39`](https://github.com/izelnakri/gitprint/commit/6e28b39c5526c95d5142e7da6866f1ad79b58007))

### Features
- Add Makefile, release tooling, async save_pdf, and crates.io metadata — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`fe67c11`](https://github.com/izelnakri/gitprint/commit/fe67c11475df238ce575391230eed3848b78f904))

### Refactoring
- Migrate to anyhow, update all deps, parallelise pipeline, add doctests — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`9581c08`](https://github.com/izelnakri/gitprint/commit/9581c08476087078f96355bdabd93d1e14d42259))

## [0.1.2] - 2026-02-18

### Features
- Now gitprint shows more metadata of files — 2026-02-18 by [@izelnakri](https://github.com/izelnakri) ([`514f0cb`](https://github.com/izelnakri/gitprint/commit/514f0cbc7adbd6c4d467ae1e592c92729212953a))


