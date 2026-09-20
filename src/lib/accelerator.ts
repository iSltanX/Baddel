/**
 * Shortcuts are stored the way Rust's `Shortcut::from_str` reads them —
 * "Alt+Shift+Space" — and shown the way macOS prints them: "⌥ ⇧ Space".
 * Keep the key names in step with `pretty_key` in `src-tauri/src/shortcuts.rs`.
 */

const MODIFIER_CODES = new Set([
  'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight',
  'ControlLeft', 'ControlRight', 'MetaLeft', 'MetaRight',
  'CapsLock', 'Fn',
])

/** Builds an accelerator from a keydown, or null when it is not one yet. */
export function fromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_CODES.has(event.code)) return null
  const parts: string[] = []
  if (event.ctrlKey) parts.push('Control')
  if (event.altKey) parts.push('Alt')
  if (event.shiftKey) parts.push('Shift')
  if (event.metaKey) parts.push('Command')
  // A bare letter would swallow that key everywhere in the system.
  if (parts.length === 0) return null
  parts.push(event.code)
  return parts.join('+')
}

/** The keycaps to draw for an accelerator, in the order macOS prints them. */
export function toGlyphs(accelerator: string): string[] {
  if (!accelerator) return []
  const modifiers: string[] = []
  let key = ''
  for (const part of accelerator.split('+')) {
    switch (part.toLowerCase()) {
      case 'control': case 'ctrl': modifiers.push('⌃'); break
      case 'alt': case 'option': modifiers.push('⌥'); break
      case 'shift': modifiers.push('⇧'); break
      case 'command': case 'cmd': case 'super': case 'meta': modifiers.push('⌘'); break
      default: key = part
    }
  }
  return [...modifiers, prettyKey(key)]
}

const NAMED: Record<string, string> = {
  space: 'Space',
  enter: '↩', numpadenter: '↩',
  escape: '⎋',
  backspace: '⌫',
  delete: '⌦',
  tab: '⇥',
  arrowleft: '←', arrowright: '→', arrowup: '↑', arrowdown: '↓',
  comma: ',', period: '.', slash: '/', backslash: '\\',
  semicolon: ';', quote: "'", backquote: '`',
  minus: '-', equal: '=',
  bracketleft: '[', bracketright: ']',
}

function prettyKey(code: string): string {
  const lower = code.toLowerCase()
  if (lower in NAMED) return NAMED[lower]
  return code.replace(/^Key/, '').replace(/^Digit/, '').toUpperCase()
}
