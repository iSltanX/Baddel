#!/usr/bin/env python3
"""Checks the links and images of the public documents before they go on GitHub.

    python3 scripts/check-docs.py            relative paths, #anchors, and the RTL rules of README.md
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


def rtl_problems(path: Path) -> list:
    """Arabic README rules, learnt from how GitHub actually renders it.

    GitHub gives paragraphs, lists and headings dir="auto" (the first strong letter
    decides) but gives tables and <details> nothing, so the document lives inside
    <div dir="rtl">. What that leaves fragile:
      - a block whose first letter is Latin turns LTR and scrambles its Arabic;
      - inline code that starts or ends with punctuation, or holds Arabic, flips its
        ends inside RTL text unless wrapped in <span dir="ltr">;
      - Arabic inside a fenced block loses its joining in the monospace face;
      - an alert (> [!NOTE]…) inside a <div> is not rendered as an alert;
      - shields.io stretches badge text to a computed width, which breaks Arabic.
    """
    found, fence, stack = [], False, []
    strong = lambda text: next((("L" if unicodedata.bidirectional(c) == "L" else "R")
                               for c in text if unicodedata.bidirectional(c) in ("L", "R", "AL")), None)
    arabic = lambda text: any(unicodedata.bidirectional(c) in ("R", "AL") for c in text)
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        where = f"{path.name}:{number}"
        bare = line.strip().lstrip(">").strip()
        if bare.startswith("```"):
            fence = not fence
            continue
        if fence:
            if arabic(line):
                found.append(f"{where}: Arabic inside a code block (use a table)")
            continue
        for tag in re.findall(r"<div\b[^>]*>|</div>", line):
            if tag == "</div>":
                stack and stack.pop()
            else:
                stack.append('dir="rtl"' in tag)
        if re.match(r"\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]", bare) and any(stack):
            found.append(f"{where}: alert inside a <div> is not rendered")
        if "img.shields.io" in line and re.search(r"%D[89]", line):
            found.append(f"{where}: Arabic badge text — shields.io stretches it and breaks the joining")
        for match in re.finditer(r"(?<!`)`([^`]+)`(?!`)", line):
            code = match.group(1)
            edge = lambda c: unicodedata.bidirectional(c) not in ("L", "EN", "R", "AL")
            if (edge(code[0]) or edge(code[-1]) or arabic(code)) \
                    and not line[:match.start()].endswith('<span dir="ltr">'):
                found.append(f"{where}: wrap `{code}` in <span dir=\"ltr\">")
        text = re.sub(r"^\s*([-*+]|\d+\.)\s+", "", bare)
        text = re.sub(r"\]\([^)]*\)|<[^>]+>", "", text)
        if text and not bare.startswith(("|", "<", "[![", "[!")) and strong(text) == "L":
            found.append(f"{where}: starts with a Latin word, GitHub will lay it out LTR")
    return found


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
        if name == "README.md":
            problems += rtl_problems(path)
    for problem in problems:
        print("✗", problem)
    print(f"{checked} links checked, {len(problems)} problem(s){' (online)' if check_online else ''}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
