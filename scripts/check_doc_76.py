#! /usr/bin/env python3
"""
Verify DOC_76: public documentation comment lines are at most 76 characters.

Code blocks inside doc comments (``` ... ```) are exempt, matching Synesis
Information Systems' internal project standards.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


DOC_LINE = re.compile(r"^\s*(//!|///)")

PASS = "\N{WHITE HEAVY CHECK MARK}"  # ✅
FAIL = "\N{CROSS MARK}"  # ❌


def iter_doc_violations(path: Path) -> list[str]:
    violations: list[str] = []
    in_codeblock = False

    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.rstrip()

        if DOC_LINE.match(stripped) and re.search(r"\s*```\s*$", stripped):
            in_codeblock = not in_codeblock
            continue

        if in_codeblock or not DOC_LINE.match(stripped):
            continue

        if len(stripped) > 76:
            violations.append(
                f"{path}:{line_no} ({len(stripped)} chars): {stripped}"
            )

    return violations


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    errors: list[str] = []

    for path in sorted(root.rglob("*.rs")):
        if "target" in path.parts:
            continue
        errors.extend(iter_doc_violations(path))

    if errors:
        print(
            f"{FAIL} DOC_76 violations (doc comment lines must be <= 76 characters):",
            file=sys.stderr,
        )
        print("\n".join(f"  {FAIL} {error}" for error in errors), file=sys.stderr)
        return 1

    print(f"{PASS} DOC_76: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
