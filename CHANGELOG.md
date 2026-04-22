## [0.3.1] - 2026-04-22

### features

- add v2 collateral builders

### miscellaneous

- update gitignore
- bump version to v0.3.1
## [0.3.0] - 2026-04-19

### features

- add generic token and polymarket call builders
- add non-empty call bundles and complete polymarket approval helpers
- add safe drafts and feature-gated relayer client
- add builder-authenticated relayer client

### refactor

- [**breaking**] make auth and sign structs bon-builder-only
- [**breaking**] typed nonce, transaction id, and state at client boundary
- [**breaking**] wrap remaining primitives in newtypes and fix module item ordering
- [**breaking**] reset crate for safe-first redesign
- [**breaking**] replace public client dto types with domain models
- align module usage

### documentation

- add public api documentation
- update changelog for 0.3.0

### testing

- parameterize pack-v and config-rejects suites with rstest

### miscellaneous

- run cargo fmt and add PolyrelError Cow-payload constructors
- bump version to 0.3.0-alpha
- remove client_submit.rs example
## [0.2.1] - 2026-04-11

### features

- add neg-risk adapter approval helpers

### miscellaneous

- bump version to 0.2.1
- update CHANGELOG for v0.2.1
## [0.2.0] - 2026-04-06

### features

- sign Safe creation internally and add deployment preflight check
- export calldata builders and document batch approval workflow

### documentation

- update CHANGELOG.md for v0.2.0

### miscellaneous

- bump version to 0.2.0
## [0.1.0] - 2026-04-05

### features

- add polyrel relayer client crate

### documentation

- add crate-level usage examples with tested doc blocks
- add docs.rs link to README
- add CHANGELOG.md for v0.1.0

### miscellaneous

- add gitignore
- add license
- add README
- add rustfmt
- add target directory to gitignore
- bump version to 0.1.0
- add git-cliff configuration
- fix README
- add package metadata to Cargo.toml
