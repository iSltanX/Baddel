#!/usr/bin/env python3
"""Acceptance B: what only real key presses can show, after the 2026-10 fixes.

⚠️ Sends real key presses. Run only with the owner's approval in the session and the owner away
from the machine, as scripts/device-test/ requires; the runner needs Accessibility (granted to
the Claude app for computer use, or to the terminal running this). Every key goes through
matrix.py's `key`, which first checks that the target app is in front *and* holds the keyboard,
and aborts otherwise.

Needs the signed debug build running with its stderr in $BADDEL_LOG and ⌥⇧Z bound to undo
(device.sh starts it so), a new TextEdit document, Safari showing scripts/device-test/page.html
(its textarea focused), and the ABC layout active. Nothing is ever typed or pasted into anything
else: the second app is the repository's own test page, never a note or document of the owner's.

    device.py regression        the four standard cases in TextEdit: the focus guard must not
                                stop a conversion where nothing moved
    device.py switch            GEMINI-003: the hotkey's modifiers still held, Safari (the test
                                page) comes to the front; nothing may change in either
    device.py undo-switch       the same for undo
    device.py window            typing, ⌘A ⌘C and ⌘V in Baddel's own report window, an accessory
                                app with no menu bar of its own
"""
import os, subprocess, sys, time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../scripts/device-test"))
import matrix  # noqa: E402
from matrix import KEY_A, KEY_C, KEY_V, KEY_SPACE, NotFront, ensure_front, get_clip, key, log_since, osa, outcomes_in, set_clip  # noqa: E402

TEXTEDIT, SAFARI, BADDEL = "com.apple.TextEdit", "com.apple.Safari", "com.isltanx.baddel"
# Where a screenshot of an abort goes (device.sh sets it), to see what had the keyboard instead.
SHOTS = os.environ.get("BADDEL_ACCEPT_SHOTS")


def where(step):
    """Who is in front and who holds the keys, for the record."""
    print(f"             {step}: front={matrix.front()} keys={matrix.focus_owner() or '?'}", flush=True)


def text_of(bundle):
    """The front document's whole text, through the pasteboard (⌘A ⌘C), the app brought forward first."""
    ensure_front(bundle)
    key(bundle, KEY_A, ("command",))
    key(bundle, KEY_C, ("command",))
    time.sleep(0.4)
    return get_clip()


def put(bundle, text):
    ensure_front(bundle)
    set_clip(text)
    key(bundle, KEY_A, ("command",))
    key(bundle, KEY_V, ("command",))
    time.sleep(0.5)


HOLD = os.path.join(os.environ.get("TMPDIR", "/tmp"), "baddel-hold-hotkey")


def hold(*args):
    """hold-hotkey.swift, compiled once: System Events lets ⌥⇧ go with the key, this does not."""
    source = os.path.join(os.path.dirname(os.path.abspath(__file__)), "hold-hotkey.swift")
    if not os.path.exists(HOLD) or os.path.getmtime(HOLD) < os.path.getmtime(source):
        subprocess.run(["swiftc", "-O", "-o", HOLD, source], check=True, capture_output=True)
    subprocess.run([HOLD, *args], check=True)


def hold_hotkey_then_switch(bundle, to, keycode=KEY_SPACE):
    """⌥⇧ and the key pressed, ⌥⇧ kept down — Baddel then waits for them to be let go — and,
    while they are held, `to` brought to the front. Then ⌥⇧ let go."""
    ensure_front(bundle, activate=False)
    hold("press", str(keycode))
    try:
        time.sleep(0.1)
        os.system(f"open -b {to}")
        time.sleep(0.3)
    finally:
        hold("release")
    time.sleep(1.5)


def switch_case(undo):
    before_te = "مرحبا اثممخ"
    put(TEXTEDIT, before_te)
    if undo:
        offset = os.path.getsize(matrix.LOG)
        matrix.hotkey(TEXTEDIT)
        time.sleep(1.5)
        before_te = text_of(TEXTEDIT)
        assert before_te.strip() == "مرحبا hello", f"setup conversion gave {before_te!r}"
    where("TextEdit ready")
    put(SAFARI, "the test page stays as it is")
    before_page = text_of(SAFARI)
    where("test page ready")
    ensure_front(TEXTEDIT)
    where("back in TextEdit")
    offset = os.path.getsize(matrix.LOG)
    # ⌥⇧Space, or ⌥⇧Z, bound to undo by device.sh for the test.
    hold_hotkey_then_switch(TEXTEDIT, SAFARI, 6 if undo else KEY_SPACE)
    where("after the switch")
    outcomes = [o for o, _ in outcomes_in(log_since(offset))]
    after_page = text_of(SAFARI)
    after_te = text_of(TEXTEDIT)
    unchanged = after_page == before_page and after_te.strip() == before_te.strip()
    stopped = outcomes in (["Failed"], [])  # FocusMoved is reported as Failed, with its reason
    label = "undo-switch" if undo else "switch"
    print(f"{label:12} {'PASS' if unchanged and stopped else 'FAIL'} outcomes={outcomes} "
          f"textedit={after_te.strip()!r} test_page_unchanged={after_page == before_page}", flush=True)


def window_case():
    """Baddel's report window (BADDEL_WINDOW=report), an accessory app's window with no menu bar of
    its own: type into the description, then ⌘A ⌘C and ⌘V. WebKit does not show the page's fields
    to an outside accessibility query here, so Tab reaches the field; a Tab that lands on the
    problem-type picker before it only moves that picker, inside this test's own window."""
    ensure_front(BADDEL)
    typed = "accessory typing works"
    copied = ""
    for _ in range(3):
        key(BADDEL, 48)  # Tab
        time.sleep(0.3)
        ensure_front(BADDEL, activate=False)
        osa(f'tell application "System Events" to keystroke "{typed}"')
        time.sleep(0.3)
        copied = text_of(BADDEL)
        if typed in copied:
            break
    set_clip(" + pasted")
    key(BADDEL, 124)  # → to the end
    key(BADDEL, KEY_V, ("command",))
    time.sleep(0.3)
    after = text_of(BADDEL)
    ok = typed in copied and after.rstrip().endswith(" + pasted")
    print(f"window       {'PASS' if ok else 'FAIL'} copied={copied!r} after_paste={after!r}", flush=True)


if __name__ == "__main__":
    case = sys.argv[1]
    saved = get_clip()
    try:
        if case == "regression":
            matrix.matrix("TextEdit", TEXTEDIT, list(matrix.CASES), 1)
        elif case == "switch":
            switch_case(undo=False)
        elif case == "undo-switch":
            switch_case(undo=True)
        elif case == "window":
            window_case()
        else:
            sys.exit(f"unknown case {case}")
    except NotFront as e:
        print(f"{case:12} ABORT {e}", flush=True)
        if SHOTS:
            subprocess.run(["screencapture", "-x", os.path.join(SHOTS, f"abort-{case}.png")])
    finally:
        set_clip(saved)
