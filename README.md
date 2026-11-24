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
- [`stateless-validator-common`](crates/stateless-validator-common) - Canonical `no_std` tests-zkevm v21.0.7 input and output schemas
- [`stateless-validator-downloader`](crates/stateless-validator-downloader) - Downloads the registered ELFs and VKs and checks their sha256
- [`stateless-validator-test`](crates/stateless-validator-test) - Runs EEST and Sepolia inputs against registry artifacts in dockerized zkVMs

### Guest Artifacts

Reth [`0.1.0-rc.9`](https://github.com/paradigmxyz/stateless/releases/tag/reth-guest-v0.1.0-rc.9) is registered on OpenVM `v2.1.0-preview`, Pico `v3.1.4`, SP1 `v6.9.0` and ZisK `v1.4.0-alpha`; its catalog ID is `1`. Ethrex [`32.0.0-rc.1`](https://github.com/lambdaclass/ethrex/releases/tag/v32.0.0-rc.1) is registered on OpenVM `v2.1.0-preview`, SP1 `v6.9.0` and ZisK `v1.4.0-alpha`; its catalog ID is `0`. Zesu [`tests@v21.0.7`](https://github.com/Consensys-Incorporated/zesu-zkvm/releases/tag/tests%40v21.0.7) is registered on OpenVM `v2.1.0-preview`, SP1 `v6.9.0` and ZisK `v1.4.0-alpha`; its catalog ID is `2`. Nimbus [`v0.4.0-alpha`](https://github.com/status-im/nimbus-zkvm-guests/releases/tag/v0.4.0-alpha) and Nethermind [`2.3.0-preview+6`](https://github.com/NethermindEth/nethermind/releases/tag/zkvm-guests-2.3.0-preview%2B6) are registered on SP1 `v6.9.0` and ZisK `v1.4.0-alpha`. Their catalog IDs are `3` and `4`. Zilkworm [`0.1.0-alpha.8`](https://github.com/erigontech/zilkworm/releases/tag/v0.1.0-alpha.8) is registered on SP1 `v6.9.0`; its catalog ID is `5`. Ere `v0.20.0` moved ZisK to `v1.4.0-alpha`, and every guest except Zilkworm publishes a ZisK `v1.4.0-alpha` ELF. Reth is the only guest with a registered Pico ELF. It targets Pico `v3.1.4` while Ere uses `v3.1.5`, which keeps the same verification keys. Zesu's Pico ELF isn't registered: its `read_input` expects an 8-byte length prefix that Ere doesn't send, so it aborts on every fixture. Nethermind's OpenVM ELF isn't registered: it targets OpenVM `v2.1.0-rv64` while Ere uses `v2.1.0-preview`, and the release has no OpenVM verification key. Every guest targets tests-zkevm `v21.0.7`. Ethrex targets Ere `v0.20.0`, Reth and Nethermind target Ere `v0.20.1`, and Zesu targets Ere `v0.20.2`. Zilkworm targets Ere `v0.19.5`. The execution tests run the tests-zkevm `v21.0.7` fixtures, and [`zkvm_execution.rs`](crates/stateless-validator-test/tests/zkvm_execution.rs) lists the ones each guest still fails.

The execution tests also run Sepolia batch `12049600-12049699` from the [Sepolia catalog](https://pub-afa6b160acfb4919bda1d0e2a00b5b77.r2.dev/testnets/sepolia/index.html). It has the highest mean gas used in the catalog, and its median block uses 187M of the 200M gas limit. Reth rejects two of its blocks. A nightly workflow runs the newest Sepolia batch on every registered guest and writes the failures to its job summary.

### Estimated Guest Cost

A pull request that changes an `elf_sha256` in [`artifact-registry.json`](artifact-registry.json) triggers the `Cost estimation benchmark` workflow. The workflow measures the old ELF and the new ELF over the same 20 Sepolia blocks, the last ones of the newest batch. It writes the difference to the job summary and, for a branch of this repository, to a pull request comment. The report is advisory and never fails the pull request. Cost units differ per zkVM.

To measure one ELF locally, pick a `batchEndBlock` from the [batch index](https://pub-afa6b160acfb4919bda1d0e2a00b5b77.r2.dev/testnets/sepolia/batches.jsonl) and run:

```bash
ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere \
  cargo run --release --package stateless-validator-test --bin zkvm_cost_estimation -- \
    --stateless-validator ethrex --zkvm openvm --zkvm-version v2.1.0-preview \
    --elf-url <url> --elf-sha256 <sha256> \
    --batch-end-block <block> --blocks 20 --output cost.json
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
