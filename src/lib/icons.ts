/**
 * Line icons in the SF Symbols manner: 24×24, 1.5pt stroke, monochrome, drawn in
 * the current text colour. Directional ones mirror with the UI; glyphs never do.
 */
export const ICON_PATHS = {
  power: 'M12 3.5v8.5M7.4 6.6a6.5 6.5 0 1 0 9.2 0',
  sidebar: 'M3.5 6.5h17v11h-17zM9 6.5v11',
  globe: 'M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17M3.5 12h17M12 3.5c-4 4.7-4 12.3 0 17M12 3.5c4 4.7 4 12.3 0 17',
  swap: 'M4 9h13l-3.5-3.5M20 15H7l3.5 3.5',
  notice: 'M4.5 7.5h15v9h-15zM8 20h8M12 16.5V20',
  speaker: 'M5 9.5h3l4-3v11l-4-3H5zM16 10a3 3 0 0 1 0 4',
  refresh: 'M20 12a8 8 0 1 1-2.4-5.7M20.5 4v4h-4',
  keyboard: 'M3 7h18v10H3zM7 10.5h.01M11 10.5h.01M15 10.5h.01M8 14h8',
  layouts: 'M3.5 7h8M7.5 5v2M10 7c0 4-3.2 7.2-6.5 8.5M5.5 11c1.6 2.7 3.9 4.3 6.5 5M13 20.5l4-10 4 10M14.5 17.5h5',
  exceptions: 'M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17M6 18L18 6',
  info: 'M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17M12 11v5.5M12 7.8h.01',
  general: 'M4 8h9M17.5 8H20M4 16h3.5M12 16h8',
  check: 'M5 12.5l4.5 4.5L19 7.5',
  checkCircle: 'M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17M8 12.3l2.6 2.6 5.4-5.4',
  warning: 'M12 4L2.8 20h18.4zM12 10v4.2M12 17h.01',
  lock: 'M5.5 11.5h13v8.5h-13zM8.5 11.5V8.5a3.5 3.5 0 0 1 7 0v3',
  plus: 'M12 5.5v13M5.5 12h13',
  minus: 'M5.5 12h13',
  close: 'M6.5 6.5l11 11M17.5 6.5l-11 11',
  chevron: 'M14.5 6l-6 6 6 6',
  noEye: 'M3.5 3.5l17 17M9.9 5.3a9 9 0 0 1 2.1-.3c5 0 9 4 10 7-.4 1.2-1.4 2.8-2.9 4.1M6.1 7.3C4.4 8.6 3.3 10.3 2.9 12c1 3 5 7 10 7 1 0 2-.2 2.9-.5M10.2 10.3a2.5 2.5 0 0 0 3.4 3.5',
  noWifi: 'M3.5 3.5l17 17M5.5 9.8A13 13 0 0 1 9 7.7M14.7 7.6a13 13 0 0 1 3.8 2.2M8.3 13a8.5 8.5 0 0 1 1.9-1.1M13.5 11.7c.8.3 1.6.8 2.2 1.4M12 17.5h.01',
  noText: 'M3.5 3.5l17 17M6 5.5h12M12 5.5v5M9.5 18.5h5M12 13.5v5',
  undo: 'M9 14.5L5 10.5l4-4M5 10.5h9a5 5 0 0 1 0 10h-2.5',
  appGeneric: 'M4.5 4.5h15v15h-15zM9 9h6v6H9z',
  external: 'M14 5h5v5M19 5l-7.5 7.5M17 14v5H5V7h5',
} as const

export type IconName = keyof typeof ICON_PATHS
