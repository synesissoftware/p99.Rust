#! /usr/bin/env python3
"""
Verify DERIVE_LAYOUT: multi-trait `#[derive(...)]` macros must be split
into separate single-trait lines, ordered alphabetically by trait name,
except tightly coupled groups (Eq/PartialEq, Ord/PartialOrd).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


COUPLED_TRAIT_GROUPS = [
    ["Eq", "PartialEq"],
    ["Ord", "PartialOrd"],
]

PASS = "\N{WHITE HEAVY CHECK MARK}"  # ✅
FAIL = "\N{CROSS MARK}"  # ❌


def lint_file(filepath: Path) -> list[str]:
    errors: list[str] = []

    lines = filepath.read_text(encoding="utf-8").splitlines()
    i = 0

    while i < len(lines):
        line = lines[i]

        if not re.match(r"^\s*#\[derive\(", line):
            i += 1
            continue

        derive_block: list[tuple[int, str]] = []
        start_line_num = i + 1

        while i < len(lines) and re.match(r"^\s*#\[derive\(", lines[i]):
            derive_block.append((i + 1, lines[i]))
            i += 1

        parsed_lines: list[tuple[int, str, str]] = []
        block_has_error = False

        for line_num, line_str in derive_block:
            match = re.search(r"#\[derive\((.*?)\)\]", line_str)

            if not match:
                continue

            traits = [
                t.strip()
                for t in match.group(1).split(",")
                if t.strip()
            ]

            if len(traits) > 1:
                if traits not in COUPLED_TRAIT_GROUPS:
                    block_has_error = True
                    allowed = ", ".join(
                        f"'{', '.join(group)}'"
                        for group in COUPLED_TRAIT_GROUPS
                    )
                    errors.append(
                        f"{filepath}:{line_num}: multi-trait derive "
                        f"'{line_str.strip()}' is not allowed "
                        f"(except coupled groups: {allowed})",
                    )
            elif len(traits) == 0:
                block_has_error = True
                errors.append(
                    f"{filepath}:{line_num}: empty derive attribute "
                    f"'{line_str.strip()}'",
                )

            sort_key = traits[0] if traits else ""
            parsed_lines.append((line_num, line_str, sort_key))

        if not block_has_error and len(parsed_lines) > 1:
            sort_keys = [item[2] for item in parsed_lines]

            if sort_keys != sorted(sort_keys):
                actual = [item[1].strip() for item in parsed_lines]
                expected = [
                    item[1].strip()
                    for item in sorted(parsed_lines, key=lambda x: x[2])
                ]
                errors.append(
                    f"{filepath}:{start_line_num}: derive attributes not "
                    f"sorted alphabetically\n"
                    f"  actual:   {actual}\n"
                    f"  expected: {expected}",
                )

    return errors


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    errors: list[str] = []

    for directory in ("src", "examples", "benches", "test"):
        base = root / directory

        if not base.is_dir():
            continue

        for path in sorted(base.rglob("*.rs")):
            if "target" in path.parts:
                continue

            errors.extend(lint_file(path))

    if errors:
        print(
            f"{FAIL} DERIVE_LAYOUT violations:",
            file=sys.stderr,
        )
        print("\n".join(f"  {FAIL} {error}" for error in errors), file=sys.stderr)
        return 1

    print(f"{PASS} DERIVE_LAYOUT: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
