#!/usr/bin/env python3
"""Check placeid Markdown for broken links, table shape, dashes, and secrets."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MD_FILES = sorted(
    p for p in ROOT.rglob("*.md") if ".git" not in p.parts and "target" not in p.parts
)
LINK_RE = re.compile(r"(?<!!)\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
SECRET_RE = re.compile(
    r"(gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|"
    r"AKIA[0-9A-Z]{16}|sk-[A-Za-z0-9]{20,})"
)


def local_target(source: Path, target: str) -> Path | None:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path = target.split("#", 1)[0]
    if not path:
        return None
    return (source.parent / path).resolve()


def check_links() -> list[str]:
    errors: list[str] = []
    for source in MD_FILES:
        text = source.read_text(encoding="utf-8")
        for match in LINK_RE.finditer(text):
            target = local_target(source, match.group(1))
            if target is not None and not target.exists():
                errors.append(
                    f"{source.relative_to(ROOT)}: broken link {match.group(1)}"
                )
    return errors


def table_cells(line: str) -> list[str]:
    stripped = line.strip()
    if stripped.startswith("|"):
        stripped = stripped[1:]
    if stripped.endswith("|"):
        stripped = stripped[:-1]
    return [cell.strip() for cell in stripped.split("|")]


def check_tables() -> list[str]:
    errors: list[str] = []
    for source in MD_FILES:
        lines = source.read_text(encoding="utf-8").splitlines()
        index = 0
        while index + 1 < len(lines):
            header = lines[index]
            separator = lines[index + 1]
            if (
                header.lstrip().startswith("|")
                and separator.lstrip().startswith("|")
                and re.fullmatch(r"\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?", separator.strip())
            ):
                expected = len(table_cells(header))
                row = index + 2
                while row < len(lines) and lines[row].lstrip().startswith("|"):
                    if len(table_cells(lines[row])) != expected:
                        errors.append(
                            f"{source.relative_to(ROOT)}:{row + 1}: table has "
                            f"{len(table_cells(lines[row]))} cells, expected {expected}"
                        )
                    row += 1
                index = row
            else:
                index += 1
    return errors


def check_text() -> list[str]:
    errors: list[str] = []
    for source in MD_FILES:
        for number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
            if "\u2014" in line or "\u2013" in line:
                errors.append(f"{source.relative_to(ROOT)}:{number}: em/en dash")
            if SECRET_RE.search(line):
                errors.append(f"{source.relative_to(ROOT)}:{number}: token literal")
    return errors


def check_requirements() -> list[str]:
    """Require each requirement defined once in SRS and referenced nowhere unknown."""
    errors: list[str] = []
    srs = (ROOT / "docs" / "SRS.md").read_text(encoding="utf-8")
    defined = set(re.findall(r"^#{2,4}\s+((?:FR|NFR)-\d{3})\b", srs, re.M))
    defined.update(re.findall(r"^\|\s*((?:FR|NFR)-\d{3})\s*\|", srs, re.M))
    id_re = re.compile(r"\b(?:FR|NFR)-\d{3}\b")
    for source in MD_FILES:
        for number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
            for req_id in id_re.findall(line):
                if req_id not in defined:
                    errors.append(
                        f"{source.relative_to(ROOT)}:{number}: unknown {req_id}"
                    )
            if re.search(r"\b(must|should|may)\b", line):
                errors.append(
                    f"{source.relative_to(ROOT)}:{number}: lowercase RFC keyword"
                )
    return errors


def main() -> int:
    errors = check_links() + check_tables() + check_text() + check_requirements()
    if errors:
        print("\n".join(errors))
        print(f"{len(errors)} issue(s)", file=sys.stderr)
        return 1
    print(f"checked {len(MD_FILES)} Markdown files: links, tables, dashes, secrets, requirement IDs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
