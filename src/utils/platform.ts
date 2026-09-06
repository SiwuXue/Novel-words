/**
 * Platform helpers shared by the Vue layer.
 *
 * Tauri mobile webviews expose the normal browser user agent, so this stays
 * independent from a native-only API and is safe to import in the browser.
 */
export const isAndroid = typeof navigator !== 'undefined'
  && /Android/i.test(navigator.userAgent)

export const isMobile = isAndroid || (typeof navigator !== 'undefined'
  && /iPhone|iPad|iPod/i.test(navigator.userAgent))
