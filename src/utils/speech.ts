/**
 * Word pronunciation via Youdao Dictionary API.
 *
 * URL pattern: https://dict.youdao.com/dictvoice?audio={word}&type={1|2}
 *   type=1 → British English (uk)
 *   type=2 → American English (us)
 *
 * Uses HTML5 Audio + Tauri webview's relaxed CSP (configured as null in
 * tauri.conf.json) so cross-origin audio just works. No Tauri HTTP plugin
 * or capability entries needed.
 */

export type SpeechAccent = 'uk' | 'us'

let currentAudio: HTMLAudioElement | null = null

function buildUrl(word: string, accent: SpeechAccent): string {
  const type = accent === 'uk' ? 1 : 2
  const encoded = encodeURIComponent(word.trim())
  return `https://dict.youdao.com/dictvoice?audio=${encoded}&type=${type}`
}

/** Stop any playback currently in progress. */
export function stopSpeaking(): void {
  if (currentAudio) {
    currentAudio.pause()
    currentAudio.currentTime = 0
    currentAudio = null
  }
}

/** Play the given word via Youdao. If another playback is in-flight, it is
 * stopped first so the new word wins (no overlap). */
export function speakWord(word: string, accent: SpeechAccent = 'us'): void {
  if (!word) return
  stopSpeaking()
  const audio = new Audio(buildUrl(word, accent))
  // Keep the reference alive at module level so calling speakWord again can
  // interrupt the previous playback.
  currentAudio = audio
  audio.onended = () => {
    if (currentAudio === audio) currentAudio = null
  }
  audio.onerror = () => {
    if (currentAudio === audio) currentAudio = null
    // Network failure / 404 / CORS-blocked (shouldn't happen with CSP=null).
    console.warn(`[speech] playback failed for "${word}"`)
  }
  // Play is async; swallow the rejection so callers don't need try/catch.
  void audio.play().catch((e) => {
    console.warn(`[speech] play() rejected for "${word}":`, e)
    if (currentAudio === audio) currentAudio = null
  })
}
