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
- [`stateless-validator-common`](crates/stateless-validator-common) - Canonical `no_std` tests-zkevm v21.0.6 input and output schemas
- [`stateless-validator-downloader`](crates/stateless-validator-downloader) - Downloads the registered ELFs and VKs and checks their sha256
- [`stateless-validator-test`](crates/stateless-validator-test) - Runs EEST and rolling devnet inputs against registry artifacts in dockerized zkVMs

### Guest Artifacts

Ethrex [`30.0.1`](https://github.com/lambdaclass/ethrex/releases/tag/v30.0.1), Reth [`0.1.0-rc.6`](https://github.com/paradigmxyz/stateless/releases/tag/reth-guest-v0.1.0-rc.6) and Zesu [`tests@v21.0.5`](https://github.com/Consensys-Incorporated/zesu-zkvm/releases/tag/tests%40v21.0.5) are registered on OpenVM `v2.1.0-preview`, SP1 `v6.9.0`, and ZisK `v1.3.2-alpha`. Their catalog IDs are `0`, `1` and `2`. Nimbus [`v0.3.1-alpha`](https://github.com/status-im/nimbus-zkvm-guests/releases/tag/v0.3.1-alpha) and Nethermind [`2.3.0-preview+3`](https://github.com/NethermindEth/nethermind/releases/tag/zkvm-guests-2.3.0-preview%2B3) are registered on SP1 `v6.9.0` and ZisK `v1.3.2-alpha`. Their catalog IDs are `3` and `4`. Zilkworm [`0.1.0-alpha.7`](https://github.com/erigontech/zilkworm/releases/tag/v0.1.0-alpha.7) is registered on SP1 `v6.9.0`; its catalog ID is `5`. Ethrex, Reth, Nimbus and Zilkworm target tests-zkevm `v21.0.6`, and Ethrex, Reth and Zilkworm also target Ere `v0.19.5`. Zesu targets tests-zkevm `v21.0.5` and Nethermind `v21.0.1`, and both target Ere `v0.19.5`. The execution tests run the tests-zkevm `v21.0.6` fixtures, and [`zkvm_execution.rs`](crates/stateless-validator-test/tests/zkvm_execution.rs) lists the ones each guest still fails.

Devnet runs are paused until Sepolia blocks exist in the engine format. The `glamsterdam-devnet-8` blocks use the tests-zkevm `v0.8.4` input layout, which v21 guests reject. Until then, the pinned 10-block pull request test is ignored, and the devnet workflow, which runs the latest 100 blocks from the rolling catalog, runs only on manual dispatch.

### Estimated Guest Cost

The benchmark is paused, like the devnet runs, until Sepolia blocks exist in the engine format.

When active, a pull request that changes an `elf_sha256` in [`artifact-registry.json`](artifact-registry.json) triggers the `Cost estimation benchmark` workflow. The workflow measures the old ELF and the new ELF over the same 100 `glamsterdam-devnet-8` blocks. It writes the difference to the job summary and, for a branch of this repository, to a pull request comment. The report is advisory and never fails the pull request. Cost units differ per zkVM.

To measure one ELF locally, pick a `batchEndBlock` from the [batch index](https://pub-760ad8b3dd9547539f829c1ea30f18b5.r2.dev/devnets/glamsterdam-devnet-8/batches.jsonl) and run:

```bash
ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere \
  cargo run --release --package stateless-validator-test --bin zkvm_cost_estimation -- \
    --stateless-validator ethrex --zkvm openvm --zkvm-version v2.1.0-preview \
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
