#!/usr/bin/env python3

"""Render the estimated cost difference between the base and head guest ELFs as Markdown."""

import argparse
import json
import sys
from pathlib import Path

WARN_PERCENT = 1.0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports", type=Path, required=True)
    parser.add_argument("--matrix", type=json.loads, required=True)
    return parser.parse_args()


def read_report(reports: Path, stateless_validator: str, zkvm: str, side: str) -> dict | None:
    path = reports / f"{stateless_validator}-{zkvm}-{side}.json"
    return json.loads(path.read_text()) if path.is_file() else None


def blocks(report: dict | None, field: str) -> dict[str, dict]:
    """Return the blocks of `report` that carry `field`, keyed by name."""
    return {b["name"]: b for b in report["blocks"] if field in b} if report else {}


def costs(estimated: dict[str, dict], names: set[str]) -> dict[str, int]:
    """Return the cost per component summed over the blocks in `names`."""
    totals: dict[str, int] = {}
    for name in names:
        for component, value in estimated[name]["cost"].items():
            totals[component] = totals.get(component, 0) + value
    return totals


def peak_heap(estimated: dict[str, dict]) -> int | None:
    peaks = [b["peak_heap_bytes"] for b in estimated.values() if b["peak_heap_bytes"] is not None]
    return max(peaks, default=None)


def number(value: int | None) -> str:
    return "-" if value is None else f"{value:,}"


def change(base: int | None, head: int | None) -> str:
    if not base or head is None:
        return "-"
    delta = (head - base) / base * 100
    mark = ""
    if delta > WARN_PERCENT:
        mark = " :warning:"
    elif delta < -WARN_PERCENT:
        mark = " :arrow_down:"
    return f"{delta:+.2f}%{mark}"


def render_pair(
    stateless_validator: str,
    zkvm: str,
    sides: set[str],
    base: dict | None,
    head: dict | None,
) -> tuple[str, list[str]]:
    """Return the overview row and the detail lines for one guest and zkVM pair."""
    base_estimated, head_estimated = blocks(base, "cost"), blocks(head, "cost")
    names = set(head_estimated) & set(base_estimated) if base else set(head_estimated)
    base_costs = costs(base_estimated, names) if base else {}
    head_costs = costs(head_estimated, names)
    base_total = sum(base_costs.values()) if base and names else None
    head_total = sum(head_costs.values()) if head and names else None
    reports = {"base": base, "head": head}
    failed = {side: blocks(report, "error") for side, report in reports.items()}

    if head is None:
        status = "no head report"
    elif "base" not in sides:
        status = "new guest"
    elif base is None:
        status = "no base report"
    elif not names:
        status = "no block estimated on both"
    else:
        status = change(base_total, head_total)
    if any(failed.values()):
        status += f", :warning: {sum(len(f) for f in failed.values())} failed"

    row = (
        f"| `{stateless_validator}` | `{zkvm}` | {number(base_total)} | {number(head_total)} "
        f"| {status} | {number(peak_heap(base_estimated))} -> {number(peak_heap(head_estimated))} |"
    )
    details = []
    if base and head and names:
        details += [
            f"Compared over {len(names)} blocks.",
            "",
            "| Component | Base | Head | Change |",
            "| --- | ---: | ---: | ---: |",
            *(
                f"| `{component}` | {number(base_costs.get(component))} "
                f"| {number(head_costs.get(component))} "
                f"| {change(base_costs.get(component), head_costs.get(component))} |"
                for component in sorted(set(base_costs) | set(head_costs))
            ),
            f"| **total** | **{number(base_total)}** | **{number(head_total)}** "
            f"| **{change(base_total, head_total)}** |",
            "",
        ]
    if base or head:
        details += [
            " ".join(
                f"{side.capitalize()} ELF `{report['elf_sha256'][:12]}` "
                f"targets `{report['zkvm_version']}`."
                for side, report in reports.items()
                if report
            ),
            "",
        ]
    for side, failures in failed.items():
        if failures:
            details += [
                f"Blocks that failed on the {side} ELF:",
                *(f"- `{name}`: {block['error']}" for name, block in failures.items()),
                "",
            ]
    if details:
        summary = f"<details><summary>{stateless_validator} on {zkvm}</summary>"
        details = [summary, "", *details, "</details>", ""]
    return row, details


def render(reports: Path, matrix: dict) -> str:
    """Build the whole comment body."""
    sides: dict[tuple[str, str], set[str]] = {}
    for entry in matrix["include"]:
        sides.setdefault((entry["stateless_validator"], entry["zkvm"]), set()).add(entry["side"])

    rows, details, measured = [], [], None
    for (stateless_validator, zkvm), pair_sides in sorted(sides.items()):
        base = read_report(reports, stateless_validator, zkvm, "base")
        head = read_report(reports, stateless_validator, zkvm, "head")
        measured = measured or head or base
        row, pair_details = render_pair(stateless_validator, zkvm, pair_sides, base, head)
        rows.append(row)
        details += pair_details

    lines = ["### Estimated guest cost", ""]
    if measured:
        lines += [
            f"Measured over {len(measured['blocks'])} `{measured['fixture_set']}` blocks "
            f"ending at {measured['fixture_end_block']:,}.",
            "",
        ]
    return "\n".join(
        [
            *lines,
            "| Guest | zkVM | Base | Head | Change | Peak heap (bytes) |",
            "| --- | --- | ---: | ---: | --- | --- |",
            *rows,
            "",
            *details,
        ]
    )


def main() -> None:
    args = parse_args()
    print(render(args.reports, args.matrix))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(str(error))
