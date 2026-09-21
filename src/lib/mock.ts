/**
 * Stand-ins for the Rust commands, for reviewing the screens in a browser
 * (`npm run dev`). This is the design stage: every screen, both languages, both
 * appearances, without building and launching the app each time.
 *
 * Loaded only by a development build running outside Tauri — the production
 * bundle drops it, because `import.meta.env.DEV` is replaced with `false`.
 */
import type { AppInfo, KeyboardMap, LayoutEntry, Settings } from './state.svelte'

const settings: Settings = {
  launchAtLogin: false,
  showTrayIcon: true,
  language: 'ar',
  switchInputSource: true,
  showHud: true,
  sound: false,
  autoUpdate: true,
  shortcutConvert: 'Alt+Shift+Space',
  shortcutUndo: '',
  shortcutPause: '',
  arabicLayout: '',
  latinLayout: '',
  excludedApps: ['com.apple.Terminal', 'com.googlecode.iterm2', 'com.1password.1password'],
  paused: false,
  welcomed: false,
}

const LAYOUTS: LayoutEntry[] = [
  { id: 'com.apple.keylayout.ABC', name: 'ABC', arabic: false },
  { id: 'com.apple.keylayout.US', name: 'U.S.', arabic: false },
  { id: 'com.apple.keylayout.Arabic', name: 'Arabic', arabic: true },
  { id: 'com.apple.keylayout.ArabicPC', name: 'Arabic – PC', arabic: true },
]

/** The stock macOS "Arabic" layout, which has no multi-letter key. */
const ROWS: [string, string][][] = [
  [['q', 'ض'], ['w', 'ص'], ['e', 'ث'], ['r', 'ق'], ['t', 'ف'], ['y', 'غ'], ['u', 'ع'], ['i', 'ه'], ['o', 'خ'], ['p', 'ح'], ['[', 'ج'], [']', 'د']],
  [['a', 'ش'], ['s', 'س'], ['d', 'ي'], ['f', 'ب'], ['g', 'ل'], ['h', 'ا'], ['j', 'ت'], ['k', 'ن'], ['l', 'م'], [';', 'ك'], ["'", 'ط']],
  [['z', 'ئ'], ['x', 'ء'], ['c', 'ؤ'], ['v', 'ر'], ['b', 'ز'], ['n', 'ى'], ['m', 'ة'], [',', 'و'], ['.', 'ز'], ['/', 'ظ']],
]

const keyboardMap: KeyboardMap = {
  rows: ROWS.map((row) => row.map(([latin, arabic]) => ({ latin, arabic, multi: arabic.length > 1 }))),
  arabicName: 'Arabic',
  latinName: 'ABC',
  hasMultiChar: ROWS.some((row) => row.some(([, arabic]) => arabic.length > 1)),
}

const APPS: AppInfo[] = [
  { id: 'com.apple.Terminal', name: 'Terminal', icon: null },
  { id: 'com.googlecode.iterm2', name: 'iTerm', icon: null },
  { id: 'com.1password.1password', name: '1Password', icon: null },
]

let permission = true
let apps = [...APPS]

/** A rough stand-in for the conversion engine: enough for the practice field. */
const AR_TO_EN = new Map(ROWS.flat().map(([latin, arabic]) => [arabic, latin]))

export function mockInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const answer = (value: unknown) => Promise.resolve(value as T)
  switch (command) {
    case 'get_settings':
      return answer(settings)
    case 'set_settings':
      Object.assign(settings, args?.patch)
      return answer({ ...settings })
    case 'set_shortcut': {
      const accelerator = String(args?.accelerator ?? '')
      const fields = {
        convert: 'shortcutConvert',
        undo: 'shortcutUndo',
        pause: 'shortcutPause',
      } as const
      const field = fields[args?.binding as keyof typeof fields]
      // One combination is treated as taken, so the conflict state is reviewable.
      const conflict = accelerator === 'Command+Shift+Space'
      if (field && !conflict) settings[field] = accelerator
      return answer({ settings: { ...settings }, conflict })
    }
    case 'permission_granted':
      return answer(permission)
    case 'request_permission':
      permission = !permission
      return answer(undefined)
    case 'list_layouts':
      return answer(LAYOUTS)
    case 'keyboard_map':
      return answer(keyboardMap)
    case 'excluded_apps':
      return answer(apps)
    case 'add_excluded_app':
      apps = [...apps, { id: 'com.example.demo', name: 'Demo', icon: null }]
      return answer(apps)
    case 'remove_excluded_app':
      apps = apps.filter((entry) => entry.id !== args?.id)
      return answer(apps)
    case 'convert_text': {
      const text = String(args?.text ?? '')
      const converted = [...text].map((character) => AR_TO_EN.get(character) ?? character).join('')
      return answer(converted === text ? null : converted)
    }
    case 'app_version':
      return answer('1.0.0')
    // `?update=available` (or checking, installing, failed) reviews the other states.
    case 'update_status': {
      const phase = new URLSearchParams(location.search).get('update') ?? 'upToDate'
      const version = phase === 'available' || phase === 'installing' ? '1.1.0' : null
      return answer({ phase, version, lastChecked: Date.now() - 2 * 60 * 60 * 1000 })
    }
    default:
      return answer(undefined)
  }
}
