/**
 * Line icons in the SF Symbols manner: 24×24, 1.5pt stroke, monochrome, drawn in
 * the current text colour. Directional ones mirror with the UI; glyphs never do.
 */
const CIRCLE = 'M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17'

export const ICON_PATHS = {
  // Same geometry as the `Icon/*` components in the Figma file (02 · Components).
  general: `M4 8h9.75M18.25 8H20M4 16h2.25M10.75 16H20M13.75 8a2.25 2.25 0 1 0 4.5 0 2.25 2.25 0 1 0-4.5 0M6.25 16a2.25 2.25 0 1 0 4.5 0 2.25 2.25 0 1 0-4.5 0`,
  keyboard: 'M5 6.5h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2zM7 10.5h.01M10.33 10.5h.01M13.67 10.5h.01M17 10.5h.01M8.5 14h7',
  layouts: 'M3.5 7h8M7.5 5v2M10 7c0 4-3.2 7.2-6.5 8.5M5.5 11c1.6 2.7 3.9 4.3 6.5 5M13 20.5l4-10 4 10M14.5 17.5h5',
  exceptions: `${CIRCLE}M6 18L18 6`,
  info: `${CIRCLE}M12 11v5.5M12 7.8h.01`,
  power: 'M12 3.5v8.5M7.4 6.6a6.5 6.5 0 1 0 9.2 0',
  menubar: 'M5.5 5.5h13a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2zM3.5 9.5h17M16.75 7.5h.01',
  globe: `${CIRCLE}M3.5 12h17M12 3.5c-4 4.7-4 12.3 0 17M12 3.5c4 4.7 4 12.3 0 17`,
  swap: 'M4 9h13l-3.5-3.5M20 15H7l3.5 3.5',
  notice: 'M6 15.5h12a3 3 0 0 0 0-6H6a3 3 0 0 0 0 6zM9 12.5h6',
  speaker: 'M5 9.5h3l4-3v11l-4-3H5zM15.5 9.5a3.5 3.5 0 0 1 0 5M18 7.2a7 7 0 0 1 0 9.6',
  refresh: 'M20 12a8 8 0 1 1-2.4-5.7M20.5 4v4h-4',
  accessibility: `${CIRCLE}M12 7.4h.01M7.8 10c2.8.9 5.6.9 8.4 0M12 10.7V14M12 14l-2.3 4M12 14l2.3 4`,
  check: 'M5 12.5l4.5 4.5L19 7.5',
  checkCircle: `${CIRCLE}M8 12.3l2.6 2.6 5.4-5.4`,
  warning: 'M12 4.5L3.2 19.5h17.6zM12 10.2v4.3M12 17h.01',
  lock: 'M7 11.5h10a1.5 1.5 0 0 1 1.5 1.5v5.5A1.5 1.5 0 0 1 17 20H7a1.5 1.5 0 0 1-1.5-1.5V13A1.5 1.5 0 0 1 7 11.5zM8.5 11.5V8.5a3.5 3.5 0 0 1 7 0v3',
  plus: 'M12 5.5v13M5.5 12h13',
  minus: 'M5.5 12h13',
  close: 'M7 7l10 10M17 7L7 17',
  chevronLeft: 'M14.5 6l-6 6 6 6',
  chevronRight: 'M9.5 6l6 6-6 6',
  chevronUpDown: 'M8.5 9.5L12 6l3.5 3.5M8.5 14.5L12 18l3.5-3.5',
  arrowLeft: 'M19 12H5M11 6l-6 6 6 6',
  arrowRight: 'M5 12h14M13 6l6 6-6 6',
  noEye: 'M3.5 3.5l17 17M9.9 5.3a9 9 0 0 1 2.1-.3c5 0 9 4 10 7-.4 1.2-1.4 2.8-2.9 4.1M6.1 7.3C4.4 8.6 3.3 10.3 2.9 12c1 3 5 7 10 7 1 0 2-.2 2.9-.5M10.2 10.3a2.5 2.5 0 0 0 3.4 3.5',
  noWifi: 'M3.5 3.5l17 17M5.5 9.8A13 13 0 0 1 9 7.7M14.7 7.6a13 13 0 0 1 3.8 2.2M8.3 13a8.5 8.5 0 0 1 1.9-1.1M13.5 11.7c.8.3 1.6.8 2.2 1.4M12 17.5h.01',
  noSave: 'M3.5 3.5l17 17M8 4.5h8.5L19.5 7.5v9M18 19.5H6A1.5 1.5 0 0 1 4.5 18V7M8.5 19.5v-5h5',
  undo: 'M9 14.5L5 10.5l4-4M5 10.5h9a5 5 0 0 1 0 10h-2.5',
  pause: 'M9 6.5v11M15 6.5v11',
  appGeneric: 'M8 4.5h8A3.5 3.5 0 0 1 19.5 8v8a3.5 3.5 0 0 1-3.5 3.5H8A3.5 3.5 0 0 1 4.5 16V8A3.5 3.5 0 0 1 8 4.5zM9.5 9.5h.01M14.5 9.5h.01M9.5 14.5h.01M14.5 14.5h.01',
  external: 'M14 5h5v5M19 5l-7.5 7.5M17 14v3.5A1.5 1.5 0 0 1 15.5 19h-9A1.5 1.5 0 0 1 5 17.5v-9A1.5 1.5 0 0 1 6.5 7H10',
  spinner: 'M12 3.5a8.5 8.5 0 1 1-8.5 8.5',
  shield: 'M12 3.5l7 2.5v5.5c0 4.3-2.8 7.7-7 9-4.2-1.3-7-4.7-7-9V6zM9 12l2.2 2.2L15.2 10',
  textCursor: 'M9 4.5c1.7 0 3 .8 3 2.5 0-1.7 1.3-2.5 3-2.5M9 19.5c1.7 0 3-.8 3-2.5 0 1.7 1.3 2.5 3 2.5M12 7v10',
} as const

export type IconName = keyof typeof ICON_PATHS
