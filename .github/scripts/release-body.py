#!/usr/bin/env python3

"""Generate release notes for the checksummed guests republished from the registry."""

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

REPOSITORY = "eth-act/ere-guests"
GUEST_PREFIX = "stateless-validator-"
TABLE_HEADER = (
    "| Stateless Validator | Version | zkVM | zkVM Version | Target | ELF | Program VK |",
    "| --- | --- | --- | --- | --- | --- | --- |",
)


@dataclass(frozen=True)
class Guest:
    """One registry-backed guest and zkVM pair."""

    name: str
    version: str
    zkvm: str
    zkvm_version: str
    source_url: str


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--artifacts-dir", type=Path, default=Path("artifacts"))
    parser.add_argument(
        "--artifact-registry", type=Path, default=Path("artifact-registry.json")
    )
    return parser.parse_args()


def artifact_name(guest: Guest, extension: str) -> str:
    """Return the local release asset name for `guest`."""
    return (
        f"{GUEST_PREFIX}{guest.name}-{guest.zkvm}-{guest.zkvm_version}.{extension}"
    )


def read_elf_word_size(elf_path: Path) -> int:
    """Return the ELF word size from its embedded ELF header."""
    data = elf_path.read_bytes()
    magic = data.find(b"\x7fELF")
    if magic < 0 or magic + 4 >= len(data):
        raise RuntimeError(f"no complete ELF header found in {elf_path}")
    if data[magic + 4] == 1:
        return 32
    if data[magic + 4] == 2:
        return 64
    raise RuntimeError(f"unknown ELF class {data[magic + 4]} in {elf_path}")


def registry_guests(artifact_registry: Path) -> list[Guest]:
    """Return every active registry pair, ordered by guest and zkVM."""
    registry = json.loads(artifact_registry.read_text())["stateless_validators"]
    guests = []
    for validator in sorted(registry, key=lambda entry: entry["name"]):
        for artifact in sorted(validator["artifacts"], key=lambda entry: entry["zkvm"]):
            guests.append(
                Guest(
                    validator["name"],
                    validator["version"],
                    artifact["zkvm"],
                    artifact["zkvm_version"],
                    artifact["elf_url"],
                )
            )
    if not guests:
        raise RuntimeError("artifact registry contains no guest artifacts")
    return guests


def render_row(guest: Guest, artifacts_dir: Path, release_url: str) -> str:
    """Render one registry guest row, requiring its ELF and VK to be present."""
    elf = artifact_name(guest, "elf")
    vk = artifact_name(guest, "vk")
    elf_path = artifacts_dir / elf
    vk_path = artifacts_dir / vk
    if not elf_path.is_file() or not vk_path.is_file():
        raise RuntimeError(f"republished guest {elf} or its VK is missing")

    target = f"riscv{read_elf_word_size(elf_path)}im"
    elf_cell = f"[Link]({release_url}/{elf}) / [Source]({guest.source_url})"
    return (
        f"| `{guest.name}` | `{guest.version}` | `{guest.zkvm}` "
        f"| `{guest.zkvm_version}` | `{target}` | {elf_cell} "
        f"| [Link]({release_url}/{vk}) |"
    )


def render_release_body(tag: str, artifacts_dir: Path, artifact_registry: Path) -> str:
    """Build the release-backed guest table for `tag`."""
    release_url = f"https://github.com/{REPOSITORY}/releases/download/{tag}"
    rows = [
        render_row(guest, artifacts_dir, release_url)
        for guest in registry_guests(artifact_registry)
    ]
    return "\n".join(
        [
            "## Release-backed guest programs",
            "",
            "Republished from the checksum-verified upstream artifacts in `artifact-registry.json`.",
            "",
            *TABLE_HEADER,
            *rows,
        ]
    )


def main() -> None:
    args = parse_args()
    print(render_release_body(args.tag, args.artifacts_dir, args.artifact_registry))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(str(error))
