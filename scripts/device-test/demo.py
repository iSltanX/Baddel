#!/usr/bin/env python3
"""Records the README demo (docs/assets/demo.gif) as the storyboard in Figma draws it
(08 — Landing / Documentation Assets · demo-storyboard 1–4): TextEdit, a word typed on the
Arabic layout, ⌥⇧Space, the notice, typing on.

⚠️ Run only when the owner asks for it. The intro text is set through AppleScript, not typed.
Every key press is sent only after TextEdit is verified to be in front, holding the keys, with
its document window key (matrix.py's guard plus the window check below), and each typed
character is checked in the document's text before the next. The first mismatch stops
everything, so a stray key can at most be one letter. (Posting events to TextEdit's process
alone, CGEventPostToPid, would be safer still, but TextEdit ignores such events for typing.)

Needs the debug build running with its stderr in $BADDEL_LOG, and an empty plain-text document
(`open -a TextEdit empty.txt`: one font, no ruler) as TextEdit's only window, placed by the caller.

    demo.py <out.mov> <x,y,w,h>
"""
import os, subprocess, sys, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import matrix as m  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
TE = "com.apple.TextEdit"
INTRO = "Hi team, the build is ready. "
HELLO = [(4, "ا"), (14, "ث"), (37, "م"), (37, "م"), (31, "خ")]       # h e l l o on Arabic
AGAIN = [(49, " "), (0, "a"), (5, "g"), (0, "a"), (34, "i"), (45, "n")]  # " again" on ABC


class Stop(Exception):
    pass


def select(source):
    subprocess.run(["swift", os.path.join(HERE, "select-input.swift"), source], check=True)


def text():
    return m.osa('tell application "TextEdit" to get text of document 1')


def raw_text():
    """The document's text exactly, trailing spaces included (osa() strips them)."""
    return subprocess.run(["osascript", "-e", 'tell application "TextEdit" to get text of document 1'],
                          capture_output=True, text=True, env=m.ENV).stdout.rstrip("\n")


def post(code, mods=()):
    if not textedit_has_keys():
        raise Stop("TextEdit does not hold the keys")
    m.key(TE, code, mods)


def type_checked(keys):
    for code, char in keys:
        before = raw_text()
        post(code)
        for _ in range(20):  # TextEdit may take a moment to show the letter
            time.sleep(0.04)
            after = raw_text()
            if after == before + char:
                break
        if after != before + char:
            raise Stop(f"typing {char!r} did not land in TextEdit: {after[-12:]!r}")


def textedit_has_keys():
    front = m.front() == TE and m.focus_owner() == TE
    window = m.osa('tell application "System Events" to tell process "TextEdit" to get name of window 1')
    doc = m.osa('tell application "TextEdit" to get name of document 1')
    return front and window == doc


def main(out, region):
    original = subprocess.run(["swift", os.path.join(HERE, "select-input.swift")], capture_output=True, text=True).stdout.strip()
    try:
        if text() != "":
            raise Stop("the TextEdit document is not empty")
        m.ensure_front(TE)
        if not textedit_has_keys():
            raise Stop("TextEdit's document window is not the key window")
        # The intro is set, not typed: no key press at all.
        m.osa(f'tell application "TextEdit" to set text of document 1 to "{INTRO}"')
        # Larger text the way a person would get it: zoom the view (⌘+ twice). Every run grows
        # alike, so the letters typed next match the intro.
        for _ in range(2):
            post(24, ("command",))
            time.sleep(0.3)
        # Setting the text leaves the caret at its end; the first typed letter proves it.
        select("com.apple.keylayout.Arabic")
        time.sleep(0.4)
        recorder = subprocess.Popen(["screencapture", "-v", "-x", "-V", "15", "-R", region, out])
        time.sleep(0.8)
        try:
            type_checked(HELLO)                        # 0–2 s: typed on the Arabic layout
            time.sleep(0.5)
            if not textedit_has_keys():
                raise Stop("TextEdit lost the keys before ⌥⇧Space")
            m.osa('tell application "System Events" to key code 49 using {option down, shift down}')  # 2–3 s
            time.sleep(1.2)
            if raw_text() != INTRO + "hello":
                raise Stop(f"the conversion did not happen: {text()[-12:]!r}")
            time.sleep(1.0)                            # 3–5 s: the notice confirms
            type_checked(AGAIN)                        # 5–7 s: the layout switched with you
            recorder.wait(timeout=20)
        finally:
            if recorder.poll() is None:
                recorder.terminate()
        print("recorded:", repr(text()))
    finally:
        if original:
            select(original)


if __name__ == "__main__":
    try:
        main(sys.argv[1], sys.argv[2])
    except Stop as stop:
        print("STOP:", stop)
        sys.exit(1)
