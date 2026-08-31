"""
Verify RUST_TEST_NAMING: test functions and test modules use TEST_ prefix
and SHOUTING_SNAKE_CASE, except words that name a specific Rust construct
(type, function, macro, field, etc.) which must preserve exact case.

When a construct name is embedded as a SHOUTING_SNAKE_CASE constant (or
PascalCase construct), it may be delimited with an extra underscore on
each side — e.g. HAVING__IGNORE_CASE__1.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


TEST_ATTR = re.compile(r"^\s*#\[(\w+::)?test(\(\))?\]")
FN_DEF = re.compile(r"^\s*fn\s+(\w+)")
MOD_DEF = re.compile(r"^\s*mod\s+(\w+)")
SNAKE_PART = re.compile(r"^[a-z][a-z0-9]*$")

PASS = "\N{WHITE HEAVY CHECK MARK}"  # ✅
FAIL = "\N{CROSS MARK}"  # ❌


def is_pascal_case_atom(atom: str) -> bool:
    return (
        atom[0].isupper()
        and any(c.islower() for c in atom)
        and atom.isalnum()
    )


def atom_violation(atom: str) -> str | None:
    if atom.isupper() or atom.isdigit():
        return None

    if atom[0].islower() and all(SNAKE_PART.match(part) for part in atom.split("_")):
        return None

    if is_pascal_case_atom(atom):
        return None

    return (
        f"segment '{atom}' must be SHOUTING_SNAKE_CASE, a PascalCase construct "
        "name, or a Rust snake_case identifier"
    )


def parse_padded_construct(
    segments: list[str], start: int
) -> tuple[str | None, int, list[str]]:
    """Parse __CONSTRUCT__ padding around a shouting or PascalCase atom."""
    violations: list[str] = []
    i = start

    while i < len(segments) and not segments[i]:
        i += 1
    if i >= len(segments):
        return None, i, ["empty segment padding without construct"]

    seg = segments[i]
    atom: str | None = None

    if seg.isupper() or seg.isdigit():
        parts = [seg]
        i += 1
        while i < len(segments) and segments[i] and (
            segments[i].isupper() or segments[i].isdigit()
        ):
            parts.append(segments[i])
            i += 1
        atom = "_".join(parts)
        reason = atom_violation(atom)
        if reason:
            violations.append(reason)
    elif seg[0].isupper() and is_pascal_case_atom(seg):
        atom = seg
        reason = atom_violation(seg)
        if reason:
            violations.append(reason)
        i += 1
    else:
        return None, start, ["empty segment padding without construct"]

    while i < len(segments) and not segments[i]:
        i += 1

    return atom, i, violations


def parse_name_atoms(rest: str) -> tuple[list[str], list[str]]:
    """Split a test name body into atoms; return (atoms, violations)."""
    atoms: list[str] = []
    violations: list[str] = []
    segments = rest.split("_")
    i = 0

    while i < len(segments):
        seg = segments[i]
        if not seg:
            start = i
            atom, i, viols = parse_padded_construct(segments, i)
            violations.extend(viols)
            if atom:
                atoms.append(atom)
            elif not viols:
                violations.append(f"empty segment in '{rest}'")
            if i == start:
                i += 1
            continue

        if seg.isupper() or seg.isdigit():
            reason = atom_violation(seg)
            if reason:
                violations.append(reason)
            else:
                atoms.append(seg)
            i += 1
            continue

        if seg[0].isupper():
            reason = atom_violation(seg)
            if reason:
                violations.append(reason)
            else:
                atoms.append(seg)
            i += 1
            continue

        if SNAKE_PART.match(seg):
            parts = [seg]
            i += 1
            while i < len(segments) and SNAKE_PART.match(segments[i]):
                parts.append(segments[i])
                i += 1
            atom = "_".join(parts)
            reason = atom_violation(atom)
            if reason:
                violations.append(reason)
            else:
                atoms.append(atom)
            continue

        violations.append(
            f"segment '{seg}' must be SHOUTING_SNAKE_CASE, a PascalCase construct "
            "name, or a Rust snake_case identifier"
        )
        i += 1

    return atoms, violations


def iter_name_violations(name: str) -> list[str]:
    if not name.startswith("TEST_"):
        return ["must start with 'TEST_'"]

    rest = name[len("TEST_") :]
    if not rest:
        return ["must have a name after 'TEST_'"]

    _, violations = parse_name_atoms(rest)
    return violations


def iter_test_results(path: Path, root: Path) -> list[tuple[bool, str]]:
    results: list[tuple[bool, str]] = []
    pending_test = False
    display = path.relative_to(root)

    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.rstrip()

        if TEST_ATTR.match(stripped):
            pending_test = True
            continue

        if pending_test and stripped.startswith("#["):
            continue

        fn_match = FN_DEF.match(stripped)
        if fn_match:
            name = fn_match.group(1)
            if pending_test:
                pending_test = False
                reasons = iter_name_violations(name)
                label = f"{display}:{line_no}: test function '{name}'"
                if reasons:
                    for reason in reasons:
                        results.append((False, f"{label}: {reason}"))
                else:
                    results.append((True, label))
            continue

        mod_match = MOD_DEF.match(stripped)
        if mod_match:
            pending_test = False
            name = mod_match.group(1)
            if name.startswith("TEST_"):
                reasons = iter_name_violations(name)
                label = f"{display}:{line_no}: test module '{name}'"
                if reasons:
                    for reason in reasons:
                        results.append((False, f"{label}: {reason}"))
                else:
                    results.append((True, label))
            continue

        if stripped and not stripped.startswith("#") and stripped.endswith("{"):
            pending_test = False

    return results


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    results: list[tuple[bool, str]] = []

    for path in sorted(root.rglob("*.rs")):
        if "target" in path.parts:
            continue
        results.extend(iter_test_results(path, root))

    failures = [line for ok, line in results if not ok]
    if failures:
        print(
            f"{FAIL} RUST_TEST_NAMING violations "
            "(test functions and modules must use TEST_ + SHOUTING_SNAKE_CASE):",
            file=sys.stderr,
        )
        for ok, line in results:
            mark = PASS if ok else FAIL
            print(f"  {mark} {line}", file=sys.stderr)
        return 1

    print(f"{PASS} RUST_TEST_NAMING: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
