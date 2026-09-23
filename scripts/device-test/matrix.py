#!/usr/bin/env python3
"""Phase-7 device test: drives the real hotkey in a real app and checks the result.

⚠️ Sends real key presses to the front app. Run only inside phase 7, when the user has
said they are away from the machine. Before every key press it checks that the target
app is in front and holds the keyboard focus, and stops otherwise, so a stray window —
or a system prompt that took the keys while the app stayed in front — never receives them.

Needs the debug build running with its stderr in $BADDEL_LOG (see run.sh): the outcome
and timing of each conversion are read from there. The log holds outcomes, paths and
lengths only, never text.

    matrix.py <label> <bundle-id> [cases] [repeat]      cases: word,l2a,select,extend
    matrix.py <label> <bundle-id> --expect <Outcome>    hotkey only, e.g. Excluded, Blocked
"""
import os, re, subprocess, sys, time

LOG = os.environ.get("BADDEL_LOG") or sys.exit("set BADDEL_LOG to the running app's stderr file")
ENV = dict(os.environ, LC_ALL="en_US.UTF-8", LANG="en_US.UTF-8")
SENTINEL = "BADDEL-CLIP-SENTINEL-7f3a"
KEY_A, KEY_C, KEY_V, KEY_SPACE = 0, 8, 9, 49  # key codes: layout independent, unlike keystroke

# name: (text before, select all first, hotkey presses, expected text)
CASES = {
    "word":   ("مرحبا اثممخ", False, 1, "مرحبا hello"),
    "l2a":    ("مرحبا world", False, 1, "مرحبا صخقمي"),
    "select": ("اثممخ صخقمي", True, 1, "hello world"),
    "extend": ("اثممخ صخقمي", False, 2, "hello world"),
}


class NotFront(Exception):
    pass


def osa(script):
    return subprocess.run(["osascript", "-e", script], capture_output=True, text=True, env=ENV).stdout.strip()


def front():
    # The app that receives key presses. System Events' "frontmost" can lag behind it (it
    # kept naming TextEdit while a just-launched Baddel held the keys), so ask Launch Services.
    asn = subprocess.run(["lsappinfo", "front"], capture_output=True, text=True).stdout.strip()
    info = subprocess.run(["lsappinfo", "info", "-only", "bundleid", asn], capture_output=True, text=True).stdout
    m = re.search(r'bundleID="([^"]+)"', info)
    return m.group(1) if m else ""


FOCUS_OWNER = os.path.join(os.environ.get("TMPDIR", "/tmp"), "baddel-focus-owner")


def focus_owner():
    """The app holding the keys; "" when the system cannot say (then assume the worst)."""
    source = os.path.join(os.path.dirname(os.path.abspath(__file__)), "focus-owner.swift")
    if not os.path.exists(FOCUS_OWNER) or os.path.getmtime(FOCUS_OWNER) < os.path.getmtime(source):
        subprocess.run(["swiftc", "-O", "-o", FOCUS_OWNER, source], check=True, capture_output=True)
    return subprocess.run([FOCUS_OWNER], capture_output=True, text=True).stdout.strip()


def ensure_front(bundle, activate=True):
    for _ in range(30):
        if front() == bundle and focus_owner() == bundle:
            return
        if activate:  # AppleScript's `activate` is not reliable on recent macOS; `open` is
            subprocess.run(["open", "-b", bundle])
        time.sleep(0.2)
    raise NotFront(f"{bundle} not in front (front: {front()}, keys: {focus_owner() or 'unknown'})")


def key(bundle, code, mods=()):
    ensure_front(bundle, activate=False)
    using = " using {" + ", ".join(f"{m} down" for m in mods) + "}" if mods else ""
    osa(f'tell application "System Events" to key code {code}{using}')


def set_clip(text):
    subprocess.run(["pbcopy"], input=text, text=True, env=ENV)


def get_clip():
    return subprocess.run(["pbpaste"], capture_output=True, text=True, env=ENV).stdout


def log_since(offset):
    with open(LOG, "rb") as f:
        f.seek(offset)
        return f.read().decode("utf-8", "replace")


def hotkey(bundle):
    key(bundle, KEY_SPACE, ("option", "shift"))


def outcomes_in(log):
    scale = {"µs": 0.001, "ms": 1, "s": 1000}
    return [(o, float(n) * scale[u]) for o, n, u in re.findall(r"\[baddel\] (\w+) in ([\d.]+)(µs|ms|s)", log)]


def run_case(bundle, name):
    text, select, presses, expected = CASES[name]
    ensure_front(bundle)
    set_clip(text)
    key(bundle, KEY_A, ("command",))
    key(bundle, KEY_V, ("command",))
    time.sleep(0.5)
    if select:
        key(bundle, KEY_A, ("command",))
        time.sleep(0.2)
    set_clip(SENTINEL)
    time.sleep(0.2)
    offset = os.path.getsize(LOG)
    for i in range(presses):
        hotkey(bundle)
        time.sleep(0.9 if i + 1 < presses else 1.5)
    clip_ok = get_clip() == SENTINEL
    log = log_since(offset)
    paths = sorted(set(re.findall(r"(text range path|key path \(\w+\)|selection unavailable)", log)))
    key(bundle, KEY_A, ("command",))
    key(bundle, KEY_C, ("command",))
    time.sleep(0.4)
    result = get_clip().strip()
    return result == expected, clip_ok, outcomes_in(log), paths, result, expected


def matrix(label, bundle, cases, repeat):
    for _ in range(repeat):
        for name in cases:
            try:
                ok, clip_ok, outcomes, paths, result, expected = run_case(bundle, name)
            except NotFront as e:
                print(f"{label:16} {name:7} ABORT {e}", flush=True)
                return False
            timing = " ".join(f"{o}@{ms:.0f}ms" for o, ms in outcomes)
            print(f"{label:16} {name:7} {'PASS' if ok else 'FAIL'} clip={'ok' if clip_ok else 'CHANGED'} "
                  f"{timing} {'; '.join(paths)}" + ("" if ok else f"  got={result!r} want={expected!r}"), flush=True)
    return True


def expect(label, bundle, wanted):
    try:
        ensure_front(bundle)
        offset = os.path.getsize(LOG)
        hotkey(bundle)
        time.sleep(1.2)
        still_front = front() == bundle
    except NotFront as e:
        print(f"{label:16} expect  ABORT {e}", flush=True)
        return False
    got = [o for o, _ in outcomes_in(log_since(offset))]
    ok = got == [wanted]
    print(f"{label:16} expect  {'PASS' if ok else 'FAIL'} outcome={got} want={wanted} "
          f"still_front={still_front}", flush=True)
    return ok


if __name__ == "__main__":
    label, bundle = sys.argv[1], sys.argv[2]
    saved = get_clip()  # text only; the pasteboard proper is Baddel's to restore
    try:
        if len(sys.argv) > 4 and sys.argv[3] == "--expect":
            expect(label, bundle, sys.argv[4])
        else:
            cases = sys.argv[3].split(",") if len(sys.argv) > 3 else list(CASES)
            matrix(label, bundle, cases, int(sys.argv[4]) if len(sys.argv) > 4 else 1)
    finally:
        set_clip(saved)
