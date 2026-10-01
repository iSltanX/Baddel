#!/usr/bin/env python3
"""v1.2 device tests: Convert Line, Convert to Arabic / to English, technical words, the pause
for a set time and "Exclude Current App" from the menu bar menu, and the per-app route memory.

⚠️ Sends real key presses and menu clicks, with the same guard as matrix.py: before every key
press the target app must be in front and hold the keyboard focus. Run only when the owner asks
for device tests. Needs the debug build running with its stderr in $BADDEL_LOG, the test
shortcuts bound (Control-Option-Shift-L / A / E) and an Arabic (Mac) layout enabled.

    v12.py <label> <bundle-id> <case,...>
    cases: line, line2, toar, toen, tech, pause, exclude   (and any matrix.py case)
"""
import os, re, sys, time, subprocess

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import matrix as m  # noqa: E402

KEY_L, KEY_E = 37, 14
LINE = (KEY_L, ("control", "option", "shift"))
TO_AR = (m.KEY_A, ("control", "option", "shift"))
TO_EN = (KEY_E, ("control", "option", "shift"))
AUTO = (m.KEY_SPACE, ("option", "shift"))

# name: (text, select all first, [shortcut, …], expected)
CASES = {
    # The line from its start to the caret; the line above it stays.
    "line": ("مرحبا\nsghl ugd;l", False, [LINE], "مرحبا\nسلام عليكم"),
    # A whole line typed in Arabic letters that meant English.
    "line2": ("اثممخ صخقمي اخص شقث غخع", False, [LINE], "hello world how are you"),
    # Forced direction, literally: iPhone too (`P` types `[` on Arabic Mac).
    "toar": ("iPhone", True, [TO_AR], "ه[اخرث"),
    "toen": ("اثممخ", False, [TO_EN], "hello"),
    # Automatic: the technical word stays, and does not outvote the Arabic beside it.
    "tech": ("sghl ugd;l iPhone", True, [AUTO], "سلام عليكم iPhone"),
    "tech2": ("اثممخ iPhone", True, [AUTO], "hello iPhone"),
}


def run(bundle, name):
    text, select, presses, expected = CASES[name]
    m.ensure_front(bundle)
    m.set_clip(text)
    m.key(bundle, m.KEY_A, ("command",))
    m.key(bundle, m.KEY_V, ("command",))
    time.sleep(0.5)
    if select:
        m.key(bundle, m.KEY_A, ("command",))
        time.sleep(0.2)
    m.set_clip(m.SENTINEL)
    time.sleep(0.2)
    offset = os.path.getsize(m.LOG)
    for code, mods in presses:
        m.key(bundle, code, mods)
        time.sleep(1.5)
    clip_ok = m.get_clip() == m.SENTINEL
    log = m.log_since(offset)
    m.key(bundle, m.KEY_A, ("command",))
    m.key(bundle, m.KEY_C, ("command",))
    time.sleep(0.4)
    result = m.get_clip().strip()
    return result == expected, clip_ok, m.outcomes_in(log), result, expected, log


def menu_click(item):
    """Clicks an item of Baddel's menu bar menu (a submenu item as "Parent>Item")."""
    path = item.split(">")
    script = 'tell application "System Events" to tell process "baddel"\n' \
             '  click menu bar item 1 of menu bar 2\n  delay 0.4\n'
    if len(path) == 1:
        script += f'  click menu item "{path[0]}" of menu 1 of menu bar item 1 of menu bar 2\n'
    else:
        script += f'  click menu item "{path[0]}" of menu 1 of menu bar item 1 of menu bar 2\n  delay 0.4\n' \
                  f'  click menu item "{path[1]}" of menu 1 of menu item "{path[0]}" of menu 1 of menu bar item 1 of menu bar 2\n'
    script += "end tell"
    return subprocess.run(["osascript", "-e", script], capture_output=True, text=True, env=m.ENV)


def menu_titles():
    script = 'tell application "System Events" to tell process "baddel"\n' \
             '  click menu bar item 1 of menu bar 2\n  delay 0.4\n' \
             '  set t to name of every menu item of menu 1 of menu bar item 1 of menu bar 2\n' \
             '  key code 53\n  return t\nend tell'
    return subprocess.run(["osascript", "-e", script], capture_output=True, text=True, env=m.ENV).stdout.strip()


def settings():
    import json
    path = os.path.expanduser("~/Library/Application Support/com.isltanx.baddel/settings.json")
    return json.load(open(path))["settings"]


def report(label, name, ok, detail):
    print(f"{label:12} {name:8} {'PASS' if ok else 'FAIL'} {detail}", flush=True)


def pause(label, bundle):
    m.ensure_front(bundle)
    offset = os.path.getsize(m.LOG)
    r = menu_click("أوقف بدّل مؤقتًا>15 دقيقة")
    time.sleep(1.0)
    s = settings()
    until = s.get("pausedUntil") or 0
    minutes = (until / 1000 - time.time()) / 60
    titles = menu_titles()
    notice = "notice PausedUntil" in m.log_since(offset)
    m.ensure_front(bundle)
    offset = os.path.getsize(m.LOG)
    m.hotkey(bundle)
    time.sleep(1.2)
    outcome = [o for o, _ in m.outcomes_in(m.log_since(offset))]
    ok = s["paused"] and 14 < minutes <= 15 and notice and "متوقف حتى" in titles and outcome == ["Paused"]
    report(label, "pause", ok, f"paused={s['paused']} ends_in={minutes:.1f}min notice={notice} "
           f"outcome={outcome} menu={titles!r} {r.stderr.strip()}")
    # Resume from the menu.
    menu_click("استأنف بدّل")
    time.sleep(1.0)
    s = settings()
    ok = not s["paused"] and s.get("pausedUntil") is None
    report(label, "resume", ok, f"paused={s['paused']} pausedUntil={s.get('pausedUntil')}")


def exclude(label, bundle):
    m.ensure_front(bundle)
    offset = os.path.getsize(m.LOG)
    r = menu_click("استثنِ التطبيق الحالي")
    time.sleep(1.0)
    listed = bundle in settings()["excludedApps"]
    notice = "notice ExcludedNow" in m.log_since(offset)
    m.ensure_front(bundle)
    offset = os.path.getsize(m.LOG)
    m.hotkey(bundle)
    time.sleep(1.2)
    log = m.log_since(offset)
    outcome = [o for o, _ in m.outcomes_in(log)]
    # Just confirmed, so the "stays out of" notice does not follow.
    quiet = "notice Excluded" not in log
    ok = listed and notice and outcome == ["Excluded"] and quiet
    report(label, "exclude", ok, f"listed={listed} notice={notice} outcome={outcome} quiet={quiet} {r.stderr.strip()}")


if __name__ == "__main__":
    label, bundle, cases = sys.argv[1], sys.argv[2], sys.argv[3].split(",")
    saved = m.get_clip()
    try:
        for name in cases:
            try:
                if name == "pause":
                    pause(label, bundle)
                elif name == "exclude":
                    exclude(label, bundle)
                elif name in CASES:
                    ok, clip_ok, outcomes, result, expected, log = run(bundle, name)
                    paths = sorted(set(re.findall(r"(line: text range path|line: key path \(\w+\)|text range path|key path \(\w+\))", log)))
                    timing = " ".join(f"{o}@{ms:.0f}ms" for o, ms in outcomes)
                    report(label, name, ok, f"clip={'ok' if clip_ok else 'CHANGED'} {timing} {'; '.join(paths)}"
                           + ("" if ok else f" got={result!r} want={expected!r}"))
                else:
                    m.matrix(label, bundle, [name], 1)
            except m.NotFront as e:
                print(f"{label:12} {name:8} ABORT {e}", flush=True)
                break
    finally:
        m.set_clip(saved)
