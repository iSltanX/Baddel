#!/usr/bin/env python3
"""Checks the links and images of the public documents before they go on GitHub.

    python3 scripts/check-docs.py            relative paths and #anchors
    python3 scripts/check-docs.py --online   external links too (HTTP)

Anchors follow GitHub's rule: lowercase, keep letters/marks/digits/spaces/hyphens/
underscores, spaces become hyphens. Links to this repository's own pages on github.com
are expected to fail while the repository is private.
"""
from __future__ import annotations

import re
import sys
import unicodedata
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ["README.md", "README.en.md", "CHANGELOG.md", "PRIVACY.md"]

LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)\s]+)\)|!\[[^\]]*\]\(([^)\s]+)\)")
ATTR = re.compile(r'\b(?:src|href|srcset)="([^"]+)"')
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")


def slug(heading: str) -> str:
    text = re.sub(r"<[^>]+>|`", "", heading)                  # tags and code marks
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)      # links keep their text
    text = text.replace("*", "")
    kept = []
    for ch in text.lower():
        cat = unicodedata.category(ch)
        if cat[0] in "LMN" or cat == "Pc" or ch in " -":
            kept.append(ch)
    return "".join(kept).replace(" ", "-")


def anchors(path: Path) -> set:
    found, counts, fenced = set(), {}, False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        match = None if fenced else HEADING.match(line)
        if match:
            base = slug(match.group(2))
            n = counts.get(base, 0)
            counts[base] = n + 1
            found.add(base if n == 0 else f"{base}-{n}")
    return found


def targets(text: str):
    fenced = False
    for number, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        line = re.sub(r"`[^`]*`", "", line)                      # inline code is not a link
        for match in LINK.finditer(line):
            yield number, match.group(1) or match.group(2)
        for match in ATTR.finditer(line):
            yield number, match.group(1).split()[0]


def online(url: str) -> str | None:
    request = urllib.request.Request(url, method="HEAD", headers={"User-Agent": "baddel-docs-check"})
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            return None if response.status < 400 else str(response.status)
    except Exception as error:  # noqa: BLE001 — any failure is a finding
        return str(getattr(error, "code", error))


def main() -> int:
    check_online = "--online" in sys.argv
    problems, checked = [], 0
    for name in DOCS:
        path = ROOT / name
        if not path.exists():
            problems.append(f"{name}: missing")
            continue
        own = anchors(path)
        for line, target in targets(path.read_text(encoding="utf-8")):
            checked += 1
            where = f"{name}:{line}"
            if target.startswith(("http://", "https://")):
                if check_online and (status := online(target)):
                    problems.append(f"{where}: {target} → {status}")
                continue
            if target.startswith("mailto:"):
                continue
            file_part, _, fragment = target.partition("#")
            file_path = (path.parent / file_part) if file_part else path
            if not file_path.exists():
                problems.append(f"{where}: no such file {file_part}")
                continue
            if fragment:
                wanted = urllib.request.unquote(fragment)
                have = own if file_path == path else anchors(file_path) if file_path.suffix == ".md" else set()
                if wanted not in have:
                    problems.append(f"{where}: no heading for #{wanted} in {file_path.name}")
    for problem in problems:
        print("✗", problem)
    print(f"{checked} links checked, {len(problems)} problem(s){' (online)' if check_online else ''}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
