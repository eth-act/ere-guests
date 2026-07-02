#!/usr/bin/env python3

"""Generate the GitHub release body listing compiled and republished guest programs."""

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from subprocess import DEVNULL, PIPE, STDOUT

ROOT = Path(__file__).resolve().parents[2]
TARGET_DIR = ROOT / "target"

REPOSITORY = "eth-act/ere-guests"
GUEST_PREFIX = "stateless-validator-"
COMPILED_ELS = ("ethrex", "reth")
ZKVMS = ("openvm", "sp1", "zisk")

TABLE_HEADER = (
    "| EL | EL Version | zkVM | zkVM Version | Target | ELF | Program VK |",
    "| --- | --- | --- | --- | --- | --- | --- |",
)


@dataclass(frozen=True)
class Guest:
    """A guest program keyed by execution layer and zkVM, with display versions."""

    el: str
    el_version: str
    zkvm: str
    zkvm_version: str
    source_url: str | None = None


def parse_args() -> argparse.Namespace:
    """Parses the command-line arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--artifacts-dir", type=Path, default=Path("artifacts"))
    parser.add_argument(
        "--artifact-registry", type=Path, default=Path("artifact-registry.json")
    )
    return parser.parse_args()


def run_command(
    args: list[str], stdout: int = DEVNULL, stderr: int = PIPE
) -> subprocess.CompletedProcess:
    """Runs `args` from ROOT, raising RuntimeError with captured output on failure."""
    try:
        proc = subprocess.run(args, cwd=ROOT, stdout=stdout, stderr=stderr, text=True)
    except OSError as error:
        raise RuntimeError(f"`{' '.join(args)}` could not run: {error}") from error
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout or "").strip()
        raise RuntimeError(f"`{' '.join(args)}` failed ({proc.returncode})\n{detail}")
    return proc


def read_ere_version() -> str:
    """Returns the Ere version pinned by git tag in the workspace Cargo.toml."""
    cargo_toml = (ROOT / "Cargo.toml").read_text()
    match = re.search(r'eth-act/ere"[^}]*?\btag\s*=\s*"(v[^"]+)"', cargo_toml)
    if not match:
        raise RuntimeError('no `eth-act/ere` git `tag = "vX.Y.Z"` found in Cargo.toml')
    return match.group(1)


def read_compiled_el_version(el: str) -> str:
    """Returns the EL version from the `stateless-validator-{el}` build script."""
    crate = f"{GUEST_PREFIX}{el}"
    run_command(["cargo", "clean", "--package", crate])
    proc = run_command(["cargo", "build", "-vv", "--package", crate], PIPE, STDOUT)
    match = re.search(r"cargo:rustc-env=EL_VERSION=(\S+)", proc.stdout)
    if not match:
        raise RuntimeError(f"EL_VERSION not emitted by `{crate}` build script")
    return match.group(1)


def read_zkvm_versions() -> dict[str, str]:
    """Returns zkVM SDK versions parsed from the `ere-catalog` build-script output."""
    version_file = "zkvm_sdk_version_impl.rs"
    run_command(["cargo", "build", "--package", "ere-catalog"])
    outputs = (TARGET_DIR / "debug" / "build").glob(f"ere-catalog-*/out/{version_file}")
    latest = max(outputs, key=lambda path: path.stat().st_mtime, default=None)
    if latest is None:
        raise RuntimeError(f"{version_file} not found after building ere-catalog")
    versions = re.findall(r'Self::(\w+)\s*=>\s*"([^"]+)"', latest.read_text())
    if not versions:
        raise RuntimeError(f"failed to parse zkVM versions from {version_file}")
    return {zkvm.lower(): version for zkvm, version in versions}


def read_elf_word_size(elf_path: Path) -> int:
    """Returns the ELF word size (32 or 64) read from the embedded ELF header."""
    data = elf_path.read_bytes()
    magic = data.find(b"\x7fELF")
    if magic < 0:
        raise RuntimeError(f"no ELF magic found in {elf_path}")
    ei_class = data[magic + 4]
    if ei_class == 1:
        return 32
    if ei_class == 2:
        return 64
    raise RuntimeError(f"unknown ELF class {ei_class} in {elf_path}")


def render_row(guest: Guest, artifacts_dir: Path, release_url: str) -> str | None:
    """Renders `guest` as a Markdown table row, or None when its ELF or VK is absent."""
    elf = f"{GUEST_PREFIX}{guest.el}-{guest.zkvm}.elf"
    vk = f"{GUEST_PREFIX}{guest.el}-{guest.zkvm}.vk"
    elf_path = artifacts_dir / elf
    vk_path = artifacts_dir / vk
    if not (elf_path.is_file() and vk_path.is_file()):
        return None

    target = f"riscv{read_elf_word_size(elf_path)}im"
    elf_cell = f"[Link]({release_url}/{elf})"
    if guest.source_url:
        elf_cell += f" / [Source]({guest.source_url})"
    return (
        f"| `{guest.el}` | `{guest.el_version}` "
        f"| `{guest.zkvm}` | `{guest.zkvm_version}` "
        f"| `{target}` | {elf_cell} | [Link]({release_url}/{vk}) |"
    )


def compiled_guests(zkvm_versions: dict[str, str]) -> list[Guest]:
    """Returns the COMPILED_ELS x ZKVMS guests, with versions from the build scripts."""
    el_versions = {el: read_compiled_el_version(el) for el in COMPILED_ELS}
    return [
        Guest(el, el_versions[el], zkvm, zkvm_versions[zkvm])
        for el in COMPILED_ELS
        for zkvm in ZKVMS
    ]


def republished_guests(
    artifact_registry: Path, zkvm_versions: dict[str, str]
) -> list[Guest]:
    """Returns the registry guests, ordered by key, with zkVM versions from the SDK."""
    registry = json.loads(artifact_registry.read_text())["stateless_validator_elf"]
    guests = []
    for key, entry in sorted(registry.items()):
        el, zkvm = key.rsplit("-", 1)
        guests.append(
            Guest(el, entry["el_version"], zkvm, zkvm_versions[zkvm], entry["url"])
        )
    return guests


def compiled_rows(
    artifacts_dir: Path, release_url: str, zkvm_versions: dict[str, str]
) -> list[str]:
    """Returns rows for compiled guests, skipping any whose artifacts are absent."""
    rendered = (
        render_row(guest, artifacts_dir, release_url)
        for guest in compiled_guests(zkvm_versions)
    )
    return [row for row in rendered if row is not None]


def republished_rows(
    artifacts_dir: Path,
    artifact_registry: Path,
    release_url: str,
    zkvm_versions: dict[str, str],
) -> list[str]:
    """Returns rows for registry guests, requiring every artifact to be present."""
    rows = []
    for guest in republished_guests(artifact_registry, zkvm_versions):
        row = render_row(guest, artifacts_dir, release_url)
        if row is None:
            raise RuntimeError(
                f"republished guest {GUEST_PREFIX}{guest.el}-{guest.zkvm}.elf is missing"
            )
        rows.append(row)
    return rows


def render_release_body(tag: str, artifacts_dir: Path, artifact_registry: Path) -> str:
    """Builds the Markdown release body for `tag`."""
    release_url = f"https://github.com/{REPOSITORY}/releases/download/{tag}"

    ere_version = read_ere_version()
    zkvm_versions = read_zkvm_versions()
    compiled = compiled_rows(artifacts_dir, release_url, zkvm_versions)
    republished = republished_rows(
        artifacts_dir, artifact_registry, release_url, zkvm_versions
    )

    body = [
        "## Compiled guest programs",
        "",
        f"Built with Ere compiler version: `{ere_version}`.",
        "",
        *TABLE_HEADER,
        *compiled,
    ]
    if republished:
        body += [
            "",
            "## Republished guest programs",
            "",
            *TABLE_HEADER,
            *republished,
        ]
    return "\n".join(body)


def main() -> None:
    args = parse_args()
    print(render_release_body(args.tag, args.artifacts_dir, args.artifact_registry))


if __name__ == "__main__":
    try:
        main()
    except RuntimeError as error:
        sys.exit(str(error))
