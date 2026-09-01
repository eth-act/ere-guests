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
    - [Estimated Guest Cost](#estimated-guest-cost)
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

Ethrex `v26.0.0-rc.2` and Reth `v0.1.0-rc.2` are currently active on OpenVM, SP1, and ZisK. Zesu keeps catalog ID `2` reserved and remains inactive until compatible, checksum-backed release artifacts pass the same test matrix.

Pull requests run a pinned 10-block `glamsterdam-devnet-8` fixture set. The daily workflow runs the latest 100 available blocks from the rolling catalog.

### Estimated Guest Cost

A pull request that changes an `elf_sha256` in [`artifact-registry.json`](artifact-registry.json) triggers the `Cost estimation benchmark` workflow. The workflow measures the old ELF and the new ELF over the same 100 `glamsterdam-devnet-8` blocks. It writes the difference to the job summary and, for a branch of this repository, to a pull request comment. The report is advisory and never fails the pull request. Cost units differ per zkVM.

To measure one ELF locally, pick a `batchEndBlock` from the [batch index](https://pub-760ad8b3dd9547539f829c1ea30f18b5.r2.dev/devnets/glamsterdam-devnet-8/batches.jsonl) and run:

```bash
ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere \
  cargo run --release --package stateless-validator-test --bin zkvm_cost_estimation -- \
    --stateless-validator reth --zkvm zisk --zkvm-version v1.1.0-alpha \
    --elf-url <url> --elf-sha256 <sha256> \
    --batch-end-block <block> --blocks 100 --output cost.json
```

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
