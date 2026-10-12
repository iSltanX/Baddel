#!/usr/bin/env python3
"""Audit helper: ar/en key parity, placeholder parity, and t('…') keys used in src/ vs defined.

Read-only. Usage: python3 audit/scripts/i18n-parity.py  (from the repository root). Exit 1 on a mismatch.
Keys built dynamically (t(`x.${y}`)) cannot be resolved statically; they are listed as prefixes.
"""
import json, pathlib, re, sys

root = pathlib.Path(__file__).resolve().parents[2]
def flat(d, p=""):
    for k, v in d.items():
        key = f"{p}{k}"
        if isinstance(v, dict):
            yield from flat(v, key + ".")
        else:
            yield key, v

loc = {n: dict(flat(json.loads((root / f"src/lib/i18n/{n}.json").read_text("utf-8")))) for n in ("ar", "en")}
ph = lambda s: sorted(set(re.findall(r"\{(\w+)\}", s))) if isinstance(s, str) else []
problems = 0
for a, b in (("ar", "en"), ("en", "ar")):
    for k in sorted(set(loc[a]) - set(loc[b])):
        print(f"MISSING in {b}: {k}"); problems += 1
for k in sorted(set(loc["ar"]) & set(loc["en"])):
    if ph(loc["ar"][k]) != ph(loc["en"][k]):
        print(f"PLACEHOLDER mismatch {k}: ar={ph(loc['ar'][k])} en={ph(loc['en'][k])}"); problems += 1
    if isinstance(loc["ar"][k], str) and not loc["ar"][k].strip():
        print(f"EMPTY ar: {k}"); problems += 1

used, dynamic = set(), set()
for f in list((root / "src").rglob("*.svelte")) + list((root / "src").rglob("*.ts")):
    s = f.read_text("utf-8")
    used |= set(re.findall(r"\bt\(\s*['\"]([\w.]+)['\"]", s))
    dynamic |= {m for m in re.findall(r"\bt\(\s*`([\w.]*)\$\{", s)}
for k in sorted(used - set(loc["en"])):
    if not any(k2.startswith(k + ".") for k2 in loc["en"]):
        print(f"USED but undefined: {k}"); problems += 1
unused = [k for k in sorted(loc["en"]) if k not in used and not any(k.startswith(p) for p in dynamic)]
print(f"keys: ar={len(loc['ar'])} en={len(loc['en'])}; used literally={len(used)}; dynamic prefixes={sorted(dynamic)}")
print(f"not referenced literally from src/ (may be used by src-tauri/menu_text.rs or dynamically): {len(unused)}")
for k in unused:
    print(f"  unreferenced: {k}")
print("RESULT:", "FAIL" if problems else "PASS", f"({problems} problems)")
sys.exit(1 if problems else 0)
