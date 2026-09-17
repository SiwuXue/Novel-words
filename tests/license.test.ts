import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent } from 'vue'
import ElementPlus from 'element-plus'
import { useLicenseStore } from '@/stores/licenseStore'
import LicenseGate from '@/components/license/LicenseGate.vue'
import LicensePanel from '@/components/license/LicensePanel.vue'
import type { LicenseStatus } from '@/types/license'
import { setLocale } from '@/i18n'

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ minimize: vi.fn(), toggleMaximize: vi.fn(), close: vi.fn() }) }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn(async () => 'C:/backup.db') }))

const active: LicenseStatus = { authorized: true, mode: 'online', code: null, plan: '30d', expiresAt: 1800000000, tokenExpiresAt: 1799999000, lastVerifiedAt: 1799990000, deviceId: 'device-test', maskedCardKey: 'CY-****-ABCD', serverUrl: 'https://license.wuyiuou.top', cardPrefix: 'CY', retryAfterSeconds: null }
const inactive: LicenseStatus = { ...active, authorized: false, mode: null, code: 'ACTIVATION_REQUIRED', plan: null, maskedCardKey: null }
const wrappers: VueWrapper[] = []
const stores: ReturnType<typeof useLicenseStore>[] = []
function store() { const result = useLicenseStore(); stores.push(result); return result }
function mountLicense(component: any, slots = {}) {
  const result = mount(component, { attachTo: document.body, slots, global: { plugins: [ElementPlus], stubs: { teleport: true, ThemeToggle: true, LanguageToggle: true, WindowControls: true } } })
  wrappers.push(result); return result
}
beforeEach(() => {
  setActivePinia(createPinia()); invoke.mockReset(); listen.mockReset(); localStorage.clear()
  listen.mockResolvedValue(vi.fn())
  invoke.mockImplementation(async cmd => cmd === 'verify_license' || cmd === 'get_license_status' ? { ...inactive } : null)
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); stores.splice(0).forEach(s => s.dispose()); document.body.innerHTML = ''; vi.useRealTimers() })

describe('local license state', () => {
  it('starts locked and verifies before allowing any workspace component to mount', async () => {
    let resolve!: (value: LicenseStatus) => void
    invoke.mockImplementation(cmd => cmd === 'verify_license' ? new Promise(done => { resolve = done }) : Promise.resolve(inactive))
    const mounted = vi.fn()
    const workspace = defineComponent({ mounted, template: '<div class="protected-page">Editor</div>' })
    const wrapper = mountLicense(LicenseGate, { default: workspace })
    expect(mounted).not.toHaveBeenCalled()
    resolve(active); await flushPromises()
    expect(wrapper.find('.protected-page').exists()).toBe(true)
    expect(mounted).toHaveBeenCalledTimes(1)
  })
  it('keeps initialization errors locked even when a separate local read returns authorized', async () => {
    invoke.mockImplementation(async cmd => { if (cmd === 'verify_license') throw { code: 'CACHE_ERROR', message: 'untrusted server text' }; return active })
    const license = store(); await license.initialize()
    expect(license.authorized).toBe(false)
    expect(license.errorCode).toBe('CACHE_ERROR')
  })
  it('never unlocks a failed initialization through the periodic local check', async () => {
    invoke.mockImplementation(async cmd => { if (cmd === 'verify_license') throw { code: 'NETWORK_UNAVAILABLE' }; return active })
    const license = store(); await license.initialize(); await license.checkLocal()
    expect(license.authorized).toBe(false)
  })
  it('locks malformed responses instead of letting a truthy field authorize the workspace', async () => {
    invoke.mockResolvedValue({ ...active, authorized: 'true' })
    const license = store(); await license.initialize()
    expect(license.authorized).toBe(false)
    expect(license.errorCode).toBe('LICENSING_UNAVAILABLE')
  })
  it('preserves mounted editing state and makes it inert when an authorization event expires', async () => {
    let changed!: (event: { payload: LicenseStatus }) => void
    listen.mockImplementation(async (_name, callback) => { changed = callback; return vi.fn() })
    invoke.mockResolvedValue(active)
    const unmounted = vi.fn()
    const wrapper = mountLicense(LicenseGate, { default: defineComponent({ unmounted, template: '<textarea class="draft">draft</textarea>' }) })
    await flushPromises(); await wrapper.get('.draft').setValue('unsaved text')
    changed({ payload: { ...inactive, code: 'LICENSE_EXPIRED' } }); await flushPromises()
    expect((wrapper.get('.draft').element as HTMLTextAreaElement).value).toBe('unsaved text')
    expect(unmounted).not.toHaveBeenCalled()
    expect(wrapper.get('.license-workspace').attributes('inert')).toBeDefined()
    expect(wrapper.get('.license-workspace').attributes('aria-hidden')).toBe('true')
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
  })
  it('does not let locked license controls trigger preserved workspace global keyboard shortcuts', async () => {
    let changed!: (event: { payload: LicenseStatus }) => void
    listen.mockImplementation(async (_name, callback) => { changed = callback; return vi.fn() })
    invoke.mockResolvedValue(active)
    const shortcut = vi.fn()
    const wrapper = mountLicense(LicenseGate, { default: '<p>Editor</p>' }); await flushPromises()
    window.addEventListener('keydown', shortcut)
    try {
      changed({ payload: { ...inactive, code: 'LICENSE_EXPIRED' } }); await flushPromises()
      await wrapper.get('input').trigger('keydown', { key: 'p', ctrlKey: true })
      expect(shortcut).not.toHaveBeenCalled()
    } finally { window.removeEventListener('keydown', shortcut) }
  })
  it('refreshes real local authorization after a failed replacement card without overwriting the previous license', async () => {
    invoke.mockImplementation(async cmd => { if (cmd === 'activate_license') throw { code: 'WRONG_PRODUCT', message: '<script>bad</script>' }; return active })
    const license = store(); await license.initialize()
    expect(await license.activate('OTHER-KEY')).toBe(false)
    expect(license.authorized).toBe(true)
    expect(license.status?.maskedCardKey).toBe(active.maskedCardKey)
    expect(license.errorCode).toBe('WRONG_PRODUCT')
    expect(invoke).toHaveBeenCalledWith('get_license_status')
  })
  it('allows only one activation request and trims its card without storing the full key', async () => {
    let resolve!: (value: LicenseStatus) => void
    invoke.mockImplementation(cmd => cmd === 'activate_license' ? new Promise(done => { resolve = done }) : Promise.resolve(inactive))
    const license = store(); await license.initialize()
    const pending = license.activate('  CY-FULL-SECRET  ')
    expect(await license.activate('CY-SECOND')).toBe(false)
    expect(invoke.mock.calls.filter(([cmd]) => cmd === 'activate_license')).toHaveLength(1)
    resolve(active); expect(await pending).toBe(true)
    expect(invoke).toHaveBeenCalledWith('activate_license', { cardKey: 'CY-FULL-SECRET' })
    expect(JSON.stringify(license.$state)).not.toContain('CY-FULL-SECRET')
    expect(JSON.stringify(localStorage)).not.toContain('CY-FULL-SECRET')
  })
  it('counts down rate limits and prevents early retry without an extra server request', async () => {
    vi.useFakeTimers()
    invoke.mockImplementation(async cmd => { if (cmd === 'activate_license') throw { code: 'RATE_LIMITED', retryAfterSeconds: 3 }; return inactive })
    const license = store(); await license.initialize(); await license.activate('CY-KEY')
    expect(license.retryRemaining).toBe(3)
    expect(await license.activate('CY-KEY')).toBe(false)
    await vi.advanceTimersByTimeAsync(2000); expect(license.retryRemaining).toBe(1)
    await vi.advanceTimersByTimeAsync(1000); expect(license.retryRemaining).toBe(0)
    expect(invoke.mock.calls.filter(([cmd]) => cmd === 'activate_license')).toHaveLength(1)
  })
  it('checks local expiry every 30 seconds and online status every 15 minutes with focus throttling', async () => {
    vi.useFakeTimers()
    const license = store(); await license.initialize(); await license.startMonitoring()
    window.dispatchEvent(new Event('focus')); document.dispatchEvent(new Event('visibilitychange')); await flushPromises()
    expect(invoke.mock.calls.filter(([cmd]) => cmd === 'verify_license')).toHaveLength(1)
    await vi.advanceTimersByTimeAsync(30000)
    expect(invoke).toHaveBeenCalledWith('get_license_status')
    window.dispatchEvent(new Event('focus')); await flushPromises()
    expect(invoke.mock.calls.filter(([cmd]) => cmd === 'verify_license')).toHaveLength(2)
    await vi.advanceTimersByTimeAsync(870000)
    expect(invoke.mock.calls.filter(([cmd]) => cmd === 'verify_license')).toHaveLength(3)
    const calls = invoke.mock.calls.length; license.dispose()
    window.dispatchEvent(new Event('focus')); await vi.advanceTimersByTimeAsync(900000)
    expect(invoke.mock.calls).toHaveLength(calls)
  })
  it('keeps a newer revocation event when an older local failure refresh completes', async () => {
    let changed!: (event: { payload: LicenseStatus }) => void
    listen.mockImplementation(async (_name, callback) => { changed = callback; return vi.fn() })
    invoke.mockResolvedValue(active)
    const license=store(); await license.initialize(); await license.startMonitoring()
    let finish!: (status: LicenseStatus)=>void
    invoke.mockImplementation(async cmd=>{if(cmd==='activate_license') throw {code:'INVALID_CARD'}; return new Promise(resolve=>{finish=resolve})})
    const replacement=license.activate('CY-BAD'); await flushPromises()
    changed({payload:{...inactive,code:'LICENSE_DISABLED'}})
    finish(active); await replacement
    expect(license.authorized).toBe(false)
    expect(license.errorCode).toBe('LICENSE_DISABLED')
  })
  it('cleans up an event subscription that resolves after disposal', async () => {
    let finish!: (stop:()=>void)=>void
    listen.mockImplementation(()=>new Promise(resolve=>{finish=resolve}))
    const license=store(), stop=vi.fn(), pending=license.startMonitoring()
    license.dispose(); finish(stop); await pending
    expect(stop).toHaveBeenCalledTimes(1)
  })
  it('does not restart a cooldown timer when a pending activation rejects after disposal', async () => {
    vi.useFakeTimers()
    let failActivation!: (error:unknown)=>void
    invoke.mockImplementation(cmd=>cmd==='activate_license' ? new Promise((_resolve,reject)=>{failActivation=reject}) : Promise.resolve(inactive))
    const license=store(); await license.initialize(); const pending=license.activate('CY-KEY')
    license.dispose(); failActivation({code:'RATE_LIMITED',retryAfterSeconds:60}); await pending
    expect(vi.getTimerCount()).toBe(0)
  })
})

describe('license panel', () => {
  it('clears successful activation input and displays only the masked card and actual plan', async () => {
    invoke.mockImplementation(async cmd => cmd === 'activate_license' ? active : inactive)
    const license = store(); await license.initialize()
    const wrapper = mountLicense(LicensePanel)
    await wrapper.get('input[autocomplete="off"]').setValue('CY-FULL-SECRET')
    await wrapper.get('form').trigger('submit'); await flushPromises()
    await wrapper.findAll('button').find(button => button.text() === '更换卡密')!.trigger('click')
    expect((wrapper.get('input[autocomplete="off"]').element as HTMLInputElement).value).toBe('')
    expect(wrapper.text()).toContain('CY-****-ABCD')
    expect(wrapper.text()).toContain('30 天')
    expect(wrapper.text()).toContain('24 小时')
  })
  it('localizes known errors without displaying arbitrary server messages', async () => {
    invoke.mockImplementation(async cmd => { if (cmd === 'activate_license') throw { code: 'INVALID_CARD', message: 'arbitrary untrusted text' }; return inactive })
    const license = store(); await license.initialize()
    const wrapper = mountLicense(LicensePanel)
    await wrapper.get('input[autocomplete="off"]').setValue('CY-BAD')
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(wrapper.text()).toContain('卡密无效')
    expect(wrapper.text()).not.toContain('arbitrary untrusted text')
  })
  it('backs up user data while locked without needing activation', async () => {
    const license = store(); await license.initialize()
    const wrapper = mountLicense(LicensePanel)
    await wrapper.findAll('button').find(button => button.text() === '备份个人数据')!.trigger('click'); await flushPromises()
    expect(invoke).toHaveBeenCalledWith('backup_database', { destPath: 'C:/backup.db' })
    expect(license.authorized).toBe(false)
  })
  it('updates backup feedback when the user switches language without redoing the backup', async () => {
    const license = store(); await license.initialize()
    const wrapper = mountLicense(LicensePanel)
    await wrapper.findAll('button').find(button => button.text() === '备份个人数据')!.trigger('click'); await flushPromises()
    await setLocale('en')
    try { expect(wrapper.text()).toContain('Personal data backed up'); expect(wrapper.text()).not.toContain('个人数据已备份') }
    finally { await setLocale('zh') }
  })
})
