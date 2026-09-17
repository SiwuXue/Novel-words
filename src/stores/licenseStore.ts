import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { LICENSE_ERROR_CODES, type LicenseStatus, type LicenseErrorCode } from '@/types/license'

const emptyStatus = (): LicenseStatus => ({ authorized: false, mode: null, code: 'ACTIVATION_REQUIRED', plan: null, expiresAt: null, tokenExpiresAt: null, lastVerifiedAt: null, deviceId: '', maskedCardKey: null, serverUrl: 'https://license.wuyiuou.top', cardPrefix: 'CY', retryAfterSeconds: null })
function errorFrom(value: unknown): { code: LicenseErrorCode; retryAfterSeconds: number | null } {
  let error = value
  if (typeof error === 'string') { try { error = JSON.parse(error) } catch { /* Never render arbitrary server messages. */ } }
  const record = error && typeof error === 'object' ? error as Record<string, unknown> : {}
  const code = LICENSE_ERROR_CODES.includes(record.code as LicenseErrorCode) ? record.code as LicenseErrorCode : 'LICENSING_UNAVAILABLE'
  const seconds = typeof record.retryAfterSeconds === 'number' && Number.isFinite(record.retryAfterSeconds) ? Math.max(0, Math.ceil(record.retryAfterSeconds)) : null
  return { code, retryAfterSeconds: seconds }
}
function readStatus(value: unknown): LicenseStatus {
  if (!value || typeof value !== 'object') throw { code: 'LICENSING_UNAVAILABLE' }
  const status = value as LicenseStatus
  const validDate = (date: unknown) => date === null || typeof date === 'number' && Number.isFinite(date) && date >= 0
  if (typeof status.authorized !== 'boolean'
    || ![null, 'online', 'offline'].includes(status.mode)
    || ![null, '7d', '30d', '365d', 'lifetime'].includes(status.plan)
    || (status.code !== null && !LICENSE_ERROR_CODES.includes(status.code))
    || !validDate(status.expiresAt) || !validDate(status.tokenExpiresAt) || !validDate(status.lastVerifiedAt) || !validDate(status.retryAfterSeconds)
    || typeof status.deviceId !== 'string' || typeof status.serverUrl !== 'string' || typeof status.cardPrefix !== 'string'
    || (status.maskedCardKey !== null && typeof status.maskedCardKey !== 'string')
    || (status.authorized && (status.mode === null || status.plan === null))) throw { code: 'LICENSING_UNAVAILABLE' }
  return { ...status }
}

export const useLicenseStore = defineStore('license', () => {
  const status = ref<LicenseStatus>(emptyStatus())
  const initialized = ref(false)
  const verified = ref(false)
  const busy = ref<'verify' | 'activate' | 'local' | null>(null)
  const authorized = computed(() => initialized.value && verified.value && status.value.authorized)
  const errorCode = ref<LicenseErrorCode | null>(null)
  const retryRemaining = ref(0)
  let retryUntil = 0
  let cooldown: ReturnType<typeof setInterval> | undefined
  let localTimer: ReturnType<typeof setInterval> | undefined
  let onlineTimer: ReturnType<typeof setInterval> | undefined
  let unlisten: UnlistenFn | undefined
  let monitoring = false
  let disposed = false
  let lifecycle = 0
  let stateRevision = 0
  let lastOnlineAttempt = -Infinity
  let initialization: Promise<void> | null = null

  function setCooldown(seconds: number | null) {
    if (!seconds || disposed) return
    retryUntil = Math.max(retryUntil, Date.now() + seconds * 1000)
    updateCooldown()
    if (!cooldown) cooldown = setInterval(updateCooldown, 1000)
  }
  function updateCooldown() {
    retryRemaining.value = Math.max(0, Math.ceil((retryUntil - Date.now()) / 1000))
    if (!retryRemaining.value && cooldown) { clearInterval(cooldown); cooldown = undefined }
  }
  function apply(next: LicenseStatus, clearError = true) {
    status.value = next
    if (clearError) errorCode.value = next.code
    if (next.code === 'RATE_LIMITED') setCooldown(next.retryAfterSeconds)
  }
  function fail(error: unknown) {
    const known = errorFrom(error)
    errorCode.value = known.code
    if (known.code === 'RATE_LIMITED') setCooldown(known.retryAfterSeconds)
  }
  async function refreshAfterFailure(initial: boolean) {
    const revision = stateRevision
    try {
      const fresh = readStatus(await invoke<LicenseStatus>('get_license_status'))
      if (revision === stateRevision) apply(initial ? { ...fresh, authorized: false, mode: null } : fresh, false)
    } catch {
      if (revision === stateRevision) status.value = { ...status.value, authorized: false, mode: null }
    }
  }
  async function verify(throttled = false): Promise<boolean> {
    updateCooldown()
    if (busy.value || retryRemaining.value || (throttled && Date.now() - lastOnlineAttempt < 30000)) return false
    const initial = !initialized.value
    const revision = stateRevision
    busy.value = 'verify'; lastOnlineAttempt = Date.now()
    try {
      const fresh = readStatus(await invoke<LicenseStatus>('verify_license'))
      verified.value = true
      if (revision === stateRevision) apply(fresh)
      initialized.value = true
      return authorized.value
    } catch (error) {
      await refreshAfterFailure(initial)
      if (revision === stateRevision) fail(error)
      initialized.value = true
      return false
    } finally { busy.value = null }
  }
  async function initialize() {
    if (initialized.value) return
    if (!initialization) initialization = verify().then(() => {})
    await initialization
  }
  async function activate(cardKey: string): Promise<boolean> {
    updateCooldown()
    if (busy.value || retryRemaining.value) return false
    const key = cardKey.trim()
    if (!key) { errorCode.value = 'INVALID_REQUEST'; return false }
    const revision = stateRevision
    busy.value = 'activate'
    try {
      const fresh = readStatus(await invoke<LicenseStatus>('activate_license', { cardKey: key }))
      verified.value = true
      if (revision === stateRevision) apply(fresh)
      initialized.value = true
      return authorized.value
    } catch (error) {
      await refreshAfterFailure(false)
      if (revision === stateRevision) fail(error)
      return false
    } finally { busy.value = null }
  }
  async function checkLocal() {
    if (busy.value) return
    const revision = stateRevision
    busy.value = 'local'
    try {
      const fresh = readStatus(await invoke<LicenseStatus>('get_license_status'))
      if (revision === stateRevision) apply(fresh, !fresh.authorized)
    } catch (error) {
      if (revision === stateRevision) { status.value = { ...status.value, authorized: false, mode: null }; fail(error) }
    } finally { busy.value = null }
  }
  function resume() { if (document.visibilityState !== 'hidden') void verify(true) }
  async function startMonitoring() {
    if (monitoring) return
    monitoring = true
    disposed = false
    const generation = ++lifecycle
    localTimer = setInterval(() => { void checkLocal() }, 30000)
    onlineTimer = setInterval(() => { void verify(true) }, 15 * 60000)
    window.addEventListener('focus', resume)
    document.addEventListener('visibilitychange', resume)
    try {
      const stop = await listen<LicenseStatus>('license-state-changed', event => {
        if (!monitoring || generation !== lifecycle) return
        stateRevision++
        try { apply(readStatus(event.payload)) } catch (error) { status.value = { ...status.value, authorized: false, mode: null }; fail(error) }
      })
      if (monitoring && generation === lifecycle) unlisten = stop
      else stop()
    } catch { /* Local polling still enforces expiry when events are unavailable. */ }
  }
  function dispose() {
    monitoring = false; disposed = true; lifecycle++
    if (localTimer) clearInterval(localTimer)
    if (onlineTimer) clearInterval(onlineTimer)
    if (cooldown) clearInterval(cooldown)
    localTimer = onlineTimer = cooldown = undefined
    window.removeEventListener('focus', resume)
    document.removeEventListener('visibilitychange', resume)
    unlisten?.(); unlisten = undefined
  }
  return { authorized, initialized, busy, status, errorCode, retryRemaining, initialize, activate, verify, checkLocal, startMonitoring, dispose }
})
