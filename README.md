# Ere Guests

A catalog, downloader, and execution harness for release-backed stateless validator guests used with [Ere](https://github.com/eth-act/ere).

## Table of Contents

- [Ere Guests](#ere-guests)
  - [Table of Contents](#table-of-contents)
  - [Supported Rust Versions (MSRV)](#supported-rust-versions-msrv)
  - [Overview](#overview)
  - [Repository Structure](#repository-structure)
    - [Workspace Crates](#workspace-crates)
    - [Guest Artifacts](#guest-artifacts)
  - [Development](#development)
    - [Formatting](#formatting)
  - [License](#license)

## Supported Rust Versions (MSRV)

The current MSRV (minimum supported Rust version) is 1.93.

## Overview

This repository republishes checksum-verified guest ELFs and program verification keys from upstream releases. The active guest and zkVM combinations are defined in [`artifact-registry.json`](artifact-registry.json).

## Repository Structure

### Workspace Crates

Located in `crates/`, these provide reusable functionality for guest programs and host:

- [`stateless-validator-catalog`](crates/stateless-validator-catalog) - Catalog of active validator kinds and registry-derived versions
- [`stateless-validator-common`](crates/stateless-validator-common) - Canonical `no_std` tests-zkevm v0.8.2 input and output schemas
- [`stateless-validator-downloader`](crates/stateless-validator-downloader) - Downloads republished ELFs and VKs from releases and workflow artifacts
- [`stateless-validator-test`](crates/stateless-validator-test) - Runs EEST and rolling devnet inputs against registry artifacts in dockerized zkVMs

### Guest Artifacts

Reth `v0.1.0-rc.2` is currently active on OpenVM, SP1, and ZisK. Ethrex keeps catalog ID `0` reserved, and Zesu keeps ID `2` reserved; they remain inactive until compatible, checksum-backed release artifacts pass the same test matrix.

Pull requests run a pinned 10-block `glamsterdam-devnet-8` fixture set. The daily workflow runs the latest 100 available blocks from the rolling catalog.

## Development

### Formatting

Formatting the workspace:

```bash
.github/scripts/cargo-fmt-all.sh
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
