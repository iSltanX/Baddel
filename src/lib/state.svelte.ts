/**
 * The bridge to Rust.
 *
 * Every preference lives in the Rust process, so the windows hold no state of
 * their own: they read what these helpers return and call back on a change. The
 * `settings` event keeps two open windows — and the menu — in step.
 */
import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

import ar from './i18n/ar.json'
import en from './i18n/en.json'

export type Language = 'ar' | 'en'
export type Binding = 'convert' | 'undo' | 'pause'

export interface Settings {
  launchAtLogin: boolean
  showTrayIcon: boolean
  language: Language
  switchInputSource: boolean
  showHud: boolean
  sound: boolean
  autoUpdate: boolean
  shortcutConvert: string
  shortcutUndo: string
  shortcutPause: string
  arabicLayout: string
  latinLayout: string
  excludedApps: string[]
  paused: boolean
  welcomed: boolean
}

export interface LayoutEntry {
  id: string
  name: string
  arabic: boolean
}

export interface KeyboardKey {
  latin: string
  arabic: string
  multi: boolean
}

export interface KeyboardMap {
  rows: KeyboardKey[][]
  arabicName: string | null
  latinName: string | null
  hasMultiChar: boolean
}

export interface AppInfo {
  id: string
  name: string
  icon: string | null
}

export type UpdatePhase = 'idle' | 'checking' | 'upToDate' | 'available' | 'installing' | 'failed'

export interface UpdateStatus {
  phase: UpdatePhase
  /** The version on offer, while available or installing. */
  version: string | null
  /** Milliseconds since the Unix epoch. */
  lastChecked: number | null
}

/** The problem types of the report window, as `report.rs` names them. */
export type Problem = 'wrong-conversion' | 'no-effect' | 'undo' | 'shortcut' | 'crash' | 'suggestion' | 'other'

/** The attached image as the report window shows it; the image itself stays in Rust. */
export interface ReportImage {
  name: string
  mime: string
  bytes: number
  width: number
  height: number
  /** `data:image/png;base64,…` */
  thumbnail: string
}

/** Everything the preview screen shows, built by Rust from the payload itself. */
export interface ReportPreview {
  json: string
  diagnostics: string
  description: string
  kind: string
  category: string | null
  appVersion: string
  osVersion: string
  arch: string
  locale: string
  test: boolean
  image: ReportImage | null
}

/** What a conversion attempt came to, as the `conversion` event names it. Never any text. */
export type Outcome =
  | 'converted'
  | 'extended'
  | 'undone'
  | 'unchanged'
  | 'no-text'
  | 'too-long'
  | 'blocked'
  | 'excluded'
  | 'paused'
  | 'no-permission'
  | 'no-layouts'
  | 'failed'

export type SendFailure =
  | { kind: 'retry' }
  | { kind: 'rate-limited'; minutes: number }
  | { kind: 'rejected'; reason: string }

const bundles: Record<Language, Record<string, unknown>> = { ar, en }

/**
 * Where a command goes. In the app this is Tauri; in a browser during development
 * it is the mock, so the screens can be designed and reviewed without a build.
 */
let invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T> = tauriInvoke

/** Everything the pages render from. Populated by {@link start}. */
export const app = $state({
  settings: null as Settings | null,
  permission: false,
  update: { phase: 'idle', version: null, lastChecked: null } as UpdateStatus,
})

/**
 * Subscribes to an app event. Outside Tauri, during development, the mock plays the app's
 * part, so the screens that react to events can be reviewed too.
 */
let subscribe: <T>(event: string, handler: (payload: T) => void) => Promise<() => void> = async (event, handler) =>
  listen(event, (message) => handler(message.payload as never))

/** Loads the initial state and subscribes to changes made elsewhere. */
export async function start(): Promise<void> {
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    const mock = await import('./mock')
    invoke = mock.mockInvoke
    subscribe = mock.mockSubscribe
    void subscribe<boolean>('permission', (granted) => (app.permission = granted))
  }
  app.settings = await invoke<Settings>('get_settings')
  app.permission = await invoke<boolean>('permission_granted')
  app.update = await invoke<UpdateStatus>('update_status')
  if (!('__TAURI_INTERNALS__' in window)) return
  await listen<Settings>('settings', (event) => {
    app.settings = event.payload
  })
  await listen<boolean>('permission', (event) => {
    app.permission = event.payload
  })
  await listen<UpdateStatus>('update', (event) => {
    app.update = event.payload
  })
}

export function language(): Language {
  return app.settings?.language ?? 'ar'
}

/** The whole UI mirrors with the interface language. */
export function direction(): 'rtl' | 'ltr' {
  return language() === 'ar' ? 'rtl' : 'ltr'
}

/**
 * Looks up a dotted key in the current language, filling `{placeholders}`.
 * A missing key returns the key itself, which makes it obvious in the window.
 */
export function t(key: string, vars?: Record<string, string | number>): string {
  let value: unknown = bundles[language()]
  for (const part of key.split('.')) {
    if (value === null || typeof value !== 'object') return key
    value = (value as Record<string, unknown>)[part]
  }
  if (typeof value !== 'string') return key
  if (!vars) return value
  return value.replace(/\{(\w+)\}/g, (match, name) => String(vars[name] ?? match))
}

export async function save(patch: Partial<Settings>): Promise<void> {
  app.settings = await invoke<Settings>('set_settings', { patch })
}

export async function setShortcut(binding: Binding, accelerator: string): Promise<boolean> {
  const result = await invoke<{ settings: Settings; conflict: boolean }>('set_shortcut', {
    binding,
    accelerator,
  })
  app.settings = result.settings
  return result.conflict
}

export const listLayouts = () => invoke<LayoutEntry[]>('list_layouts')
export const keyboardMap = () => invoke<KeyboardMap>('keyboard_map')
export const excludedApps = () => invoke<AppInfo[]>('excluded_apps')
export const addExcludedApp = () => invoke<AppInfo[]>('add_excluded_app')
export const removeExcludedApp = (id: string) => invoke<AppInfo[]>('remove_excluded_app', { id })
export const convertText = (text: string) => invoke<string | null>('convert_text', { text })
export const requestPermission = () => invoke<void>('request_permission')
export const openKeyboardSettings = () => invoke<void>('open_keyboard_settings')
export const appVersion = () => invoke<string>('app_version')
/** The answer arrives as the `update` event. */
export const checkForUpdates = () => invoke<void>('check_for_updates')
export const installUpdate = () => invoke<void>('install_update')
export const previewHud = () => invoke<void>('preview_hud')
/** Every conversion attempt, as it happens. Returns the unsubscribe function. */
export const onConversion = (handler: (outcome: Outcome) => void) => subscribe<Outcome>('conversion', handler)
export const openOnboarding = () => invoke<void>('open_onboarding')
/** True while Baddel runs from the disk image or a translocated download. */
export const appNeedsMove = () => invoke<boolean>('app_needs_move')
export const revealAppInFinder = () => invoke<void>('reveal_app_in_finder')
export const finishOnboarding = () => invoke<void>('finish_onboarding')
export const openExternal = (url: string) => invoke<void>('open_external', { url })
export const copyDiagnostics = () => invoke<void>('copy_diagnostics')
export const openReport = () => invoke<void>('open_report')
/** `null` when the user cancels the picker. Rejects with an image error code. */
export const pickReportImage = () => invoke<ReportImage | null>('report_pick_image')
export const pasteReportImage = () => invoke<ReportImage>('report_paste_image')
export const clearReportImage = () => invoke<void>('report_clear_image')
export const previewReport = (problem: Problem, description: string) =>
  invoke<ReportPreview>('report_preview', { draft: { problem, description } })
/** Resolves to the report number; rejects with a {@link SendFailure}. */
export const sendReport = () => invoke<number>('report_send')
export const copyReport = () => invoke<void>('report_copy')
export const copyReportNumber = (id: number) => invoke<void>('report_copy_number', { id })
export const setWindowTitle = (title: string) => invoke<void>('set_window_title', { title })
export const setContentHeight = (height: number) => invoke<void>('set_content_height', { height })

/** Windows are built hidden; this is what puts one on screen, once laid out. */
export async function reveal(): Promise<void> {
  await invoke<void>('reveal_window')
}

export const closeWindow = async (): Promise<void> => {
  if ('__TAURI_INTERNALS__' in window) await getCurrentWindow().close()
}
