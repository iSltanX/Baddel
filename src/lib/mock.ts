/**
 * Stand-ins for the Rust commands, for reviewing the screens in a browser
 * (`npm run dev`). This is the design stage: every screen, both languages, both
 * appearances, without building and launching the app each time.
 *
 * Loaded only by a development build running outside Tauri — the production
 * bundle drops it, because `import.meta.env.DEV` is replaced with `false`.
 */
import type { AppInfo, KeyboardMap, LayoutEntry, Settings } from './state.svelte'

/**
 * Query parameters that pick a state to review: `lang=en`, `permission=missing`,
 * `move=1` (the welcome's "move to Applications" note), `report=fail|rate-limited|rejected`,
 * `update=available|checking|installing|failed`.
 */
const query = new URLSearchParams(location.search)

const settings: Settings = {
  launchAtLogin: false,
  showTrayIcon: true,
  language: query.get('lang') === 'en' ? 'en' : 'ar',
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

/** A tiny grey PNG standing in for a screenshot. */
const MOCK_IMAGE = {
  name: '1.png',
  mime: 'image/png',
  bytes: 421_888,
  width: 1280,
  height: 800,
  thumbnail:
    'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mN88OjRfwAIQQN2f0XMWgAAAABJRU5ErkJggg==',
}

function mockPreview(draft: { problem: string; description: string }) {
  const bug = ['wrong-conversion', 'no-effect', 'undo', 'shortcut'].includes(draft.problem)
  const categories: Record<string, string> = { 'wrong-conversion': 'conversion', 'no-effect': 'no-effect', undo: 'undo', shortcut: 'shortcut' }
  const diagnostics = {
    accessibility: true,
    secure_input: false,
    layouts: { arabic: 'com.apple.keylayout.Arabic', arabic_name: 'Arabic', latin: 'com.apple.keylayout.ABC', latin_name: 'ABC', active: 'latin', source: 'live' },
    layout_choice: { arabic: 'auto', latin: 'auto' },
    recent: [{ outcome: 'converted', path: 'keys', ms: 1240, ago_s: 42 }],
    last_app: 'com.apple.mail',
    settings: { language: settings.language, switch_input_source: true },
    build: 'debug',
  }
  return {
    json: '{}',
    diagnostics: JSON.stringify(diagnostics, null, 2),
    description: draft.description.trim(),
    kind: bug ? 'bug' : draft.problem,
    category: categories[draft.problem] ?? null,
    appVersion: '1.1.0',
    osVersion: '27.0.1',
    arch: 'arm64',
    locale: settings.language,
    test: true,
    image: MOCK_IMAGE,
  }
}

let permission = query.get('permission') !== 'missing'
let apps = [...APPS]

/** The app's events, played by the mock: the permission arriving, a conversion outcome. */
const events = new EventTarget()

export async function mockSubscribe<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  const listener = (message: Event) => handler((message as CustomEvent<T>).detail)
  events.addEventListener(event, listener)
  return () => events.removeEventListener(event, listener)
}

function emit(event: string, detail: unknown) {
  events.dispatchEvent(new CustomEvent(event, { detail }))
}

/** For trying the welcome's practice states by hand, from the browser console. */
;(window as unknown as Record<string, unknown>).mockConversion = (outcome: string) => emit('conversion', outcome)

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
      // A shortcut another command has is refused by name, as in the app.
      const sameKeys = (a: string) => a.split('+').sort().join('+')
      const conflictWith =
        (accelerator &&
          (Object.keys(fields) as (keyof typeof fields)[]).find(
            (other) => other !== args?.binding && sameKeys(settings[fields[other]]) === sameKeys(accelerator),
          )) ||
        null
      // One combination is treated as taken by another app, so that state is reviewable too.
      const conflict = conflictWith !== null || accelerator === 'Command+Shift+Space'
      if (field && !conflict) settings[field] = accelerator
      return answer({ settings: { ...settings }, conflict, conflictWith })
    }
    case 'permission_granted':
      return answer(permission)
    case 'request_permission':
      // As if the user switched Baddel on in System Settings a moment later.
      setTimeout(() => {
        permission = true
        emit('permission', true)
      }, 2500)
      return answer(undefined)
    case 'app_needs_move':
      return answer(query.get('move') === '1')
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
    // The report window: `?report=fail` (or rate-limited, rejected) reviews the failure screens.
    case 'report_pick_image':
    case 'report_paste_image':
      return answer(MOCK_IMAGE)
    case 'report_preview':
      return answer(mockPreview(args?.draft as { problem: string; description: string }))
    case 'report_send': {
      const mode = query.get('report')
      if (mode === 'fail') return Promise.reject({ kind: 'retry' })
      if (mode === 'rate-limited') return Promise.reject({ kind: 'rate-limited', minutes: 42 })
      if (mode === 'rejected') return Promise.reject({ kind: 'rejected', reason: 'invalid_field: description' })
      return answer(128)
    }
    // `?update=available` (or checking, installing, failed) reviews the other states.
    case 'update_status': {
      const phase = query.get('update') ?? 'upToDate'
      const version = phase === 'available' || phase === 'installing' ? '1.1.0' : null
      return answer({ phase, version, lastChecked: Date.now() - 2 * 60 * 60 * 1000 })
    }
    default:
      return answer(undefined)
  }
}
