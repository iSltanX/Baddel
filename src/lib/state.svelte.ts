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
})

/** Loads the initial state and subscribes to changes made elsewhere. */
export async function start(): Promise<void> {
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    invoke = (await import('./mock')).mockInvoke
  }
  app.settings = await invoke<Settings>('get_settings')
  app.permission = await invoke<boolean>('permission_granted')
  if (!('__TAURI_INTERNALS__' in window)) return
  await listen<Settings>('settings', (event) => {
    app.settings = event.payload
  })
  await listen<boolean>('permission', (event) => {
    app.permission = event.payload
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
export const previewHud = () => invoke<void>('preview_hud')
export const openOnboarding = () => invoke<void>('open_onboarding')
export const finishOnboarding = () => invoke<void>('finish_onboarding')
export const openExternal = (url: string) => invoke<void>('open_external', { url })
export const setWindowTitle = (title: string) => invoke<void>('set_window_title', { title })
export const setContentHeight = (height: number) => invoke<void>('set_content_height', { height })

/** Windows are built hidden; this is what puts one on screen, once laid out. */
export async function reveal(): Promise<void> {
  await invoke<void>('reveal_window')
}

export const closeWindow = async (): Promise<void> => {
  if ('__TAURI_INTERNALS__' in window) await getCurrentWindow().close()
}
