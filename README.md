# Ere Guests

A collection of guest programs built with [Ere](https://github.com/eth-act/ere) for various zkVM platforms.

## Table of Contents

- [Ere Guests](#ere-guests)
  - [Table of Contents](#table-of-contents)
  - [Supported Rust Versions (MSRV)](#supported-rust-versions-msrv)
  - [Overview](#overview)
  - [Repository Structure](#repository-structure)
    - [Library Crates](#library-crates)
    - [Guest Programs](#guest-programs)
  - [Development](#development)
    - [Formatting](#formatting)
  - [License](#license)

## Supported Rust Versions (MSRV)

The current MSRV (minimum supported rust version) is 1.88.

## Overview

This repository contains guest programs and libraries designed to run on zkVM platforms using the Ere framework. It provides both reusable library crates and compiled guest binaries for various zkVMs.

## Repository Structure

### Library Crates

Located in `crates/`, these provide reusable functionality for guest programs and host:

- [`guest`](crates/guest) - Core guest utilities and traits for building zkVM programs
- [`block-encoding-length`](crates/block-encoding-length) - SSZ block encoding length calculation benchmark
- [`stateless-validator-ethrex`](crates/stateless-validator-ethrex) - Stateless validation using Ethrex
- [`stateless-validator-reth`](crates/stateless-validator-reth) - Stateless validation using Reth

### Guest Programs

Located in `bin/`, these are executable guest programs for various zkVMs:

- [`empty`](bin/empty) - Minimal empty program for testing
- [`panic`](bin/panic) - Minimal panic program for testing
- [`block-encoding-length`](bin/block-encoding-length) - Block encoding length calculation benchmark
- [`stateless-validator-ethrex`](bin/stateless-validator-ethrex) - Stateless validator using Ethrex
- [`stateless-validator-reth`](bin/stateless-validator-reth) - Stateless validator using Reth

## Development

### Formatting

Check formatting of the workspace and all guest programs:

```bash
.github/scripts/check-fmt.sh
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
