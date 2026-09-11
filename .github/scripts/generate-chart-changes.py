#!/usr/bin/env python3
"""Convert Keep-a-Changelog release notes to Artifact Hub changes YAML.

Reads a markdown section (as produced by extract-changelog-section.sh) on
stdin or from a file argument, and writes a YAML list to stdout suitable for
the `artifacthub.io/changes` Chart.yaml annotation.

Mapping: `### Added` -> kind added, `### Changed` -> changed,
`### Fixed` -> fixed, `### Removed` -> removed, `### Deprecated` ->
deprecated, `### Security` -> security. Anything else defaults to changed.
Only `- ` bullets become entries; other lines are ignored.
"""

import re
import sys


KIND_BY_HEADING = {
    "added": "added",
    "changed": "changed",
    "fixed": "fixed",
    "removed": "removed",
    "deprecated": "deprecated",
    "security": "security",
}


def parse_notes(text: str) -> list[tuple[str, str]]:
    kind = "changed"
    entries: list[tuple[str, str]] = []
    for line in text.splitlines():
        heading = re.match(r"^###\s+(.+?)\s*$", line)
        if heading:
            kind = KIND_BY_HEADING.get(heading.group(1).strip().lower(), "changed")
            continue
        bullet = re.match(r"^\s*-\s+(.+?)\s*$", line)
        if bullet:
            entries.append((kind, bullet.group(1)))
    return entries


def quote(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main() -> int:
    if len(sys.argv) > 2:
        print(f"Usage: {sys.argv[0]} [NOTES-FILE]", file=sys.stderr)
        return 2
    if len(sys.argv) == 2:
        with open(sys.argv[1], encoding="utf-8") as handle:
            text = handle.read()
    else:
        text = sys.stdin.read()
    entries = parse_notes(text)
    for kind, description in entries:
        print(f"- kind: {kind}")
        print(f"  description: {quote(description)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
