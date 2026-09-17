// Real PrismKey API + real Tauri commands, confined to the disposable license profile.
// Run core, restart Tauri and run restart, then run lifecycle.
import { chromium, expect as baseExpect } from '@playwright/test'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { execFileSync } from 'node:child_process'
process.env.NO_PROXY = 'localhost,127.0.0.1'
const expect = baseExpect.configure({ timeout: 30000 })
const phase = process.argv[2] || 'core'
const source = process.env.PRISMKEY_TEST_SOURCE
const python = process.env.PRISMKEY_TEST_PYTHON
if (!source || !python) throw new Error('Provide supplied PrismKey project and its test Python interpreter')
const control = action => execFileSync(python, ['tests/prepare-license-fixture.py', action, '--source', source], { stdio: 'pipe' })
const fixture = JSON.parse(await readFile('test-data/prismkey/fixture.json', 'utf8'))
const cardKey = fixture.primary.cardKey
await mkdir('test-data/native-license', { recursive: true })
const statePath = resolve('test-data/native-license/state.json')
const backupPath = resolve('test-data/native-license/backup.db')
await expect.poll(async () => {
  try { return (await fetch('http://127.0.0.1:9229/json/version')).ok } catch { return false }
}, { timeout: 180000, intervals: [1000, 3000, 10000], message: 'Wait for isolated native WebView' }).toBe(true)
const browser = await chromium.connectOverCDP('http://127.0.0.1:9229')
const page = browser.contexts()[0].pages()[0]
page.setDefaultTimeout(60000)
page.setDefaultNavigationTimeout(60000)
const invoke = (cmd, args = {}) => page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args })
const check = (value, message) => { if (!value) throw new Error(message) }
const rejection = async (cmd, args, code) => {
  // Catch inside WebView so CDP does not turn an IPC error object into a Node Error.
  const error = await page.evaluate(async ({ cmd, args }) => {
    try { await window.__TAURI_INTERNALS__.invoke(cmd, args); return null }
    catch (caught) { return typeof caught === 'string' ? JSON.parse(caught) : caught }
  }, { cmd, args })
  check(error?.code === code, `${cmd} must reject with ${code}`)
}
const checkpointOffline = async (novelId, bookId) => {
  control('online')
  const latest = await invoke('verify_license')
  check(latest.authorized && latest.mode === 'online', 'Latest successful online baseline')
  control('outage')
  const offline = await invoke('verify_license')
  check(offline.authorized && offline.mode === 'offline' && offline.tokenExpiresAt === latest.tokenExpiresAt, 'Outage uses fixed signed deadline')
  await writeFile(statePath, JSON.stringify({ novelId, bookId, expiresAt: latest.expiresAt, tokenExpiresAt: latest.tokenExpiresAt, deviceId: latest.deviceId }))
}
try {
  await page.goto('http://localhost:1420/novels', { waitUntil: 'domcontentloaded' })
  const info = await invoke('get_app_info')
  check(info.dataDir.endsWith('com.tauri-app.novel-words-license-test'), 'Requires isolated license profile')
  if (phase === 'close') {
    const selector = await page.locator('.license-gate').count() ? '.license-gate .ctrl-close' : '.license-workspace .ctrl-close'
    await page.locator(selector).click().catch(error => { if (!page.isClosed()) throw error })
    console.log('CLOSE PASS: isolated native window control closes the application')
  } else if (phase === 'init-failure') {
    await expect(page.getByRole('heading', { name: '激活词阅' })).toBeVisible()
    await rejection('get_all_novels', {}, 'LICENSING_UNAVAILABLE')
    await rejection('verify_license', {}, 'LICENSING_UNAVAILABLE')
    await invoke('backup_database', { destPath: backupPath })
    check((await readFile(backupPath)).subarray(0, 15).toString() === 'SQLite format 3', 'Initialization failure preserves backup access')
    console.log('INIT FAILURE PASS: unavailable license state blocks core but opens the gate and preserves real backup access')
  } else if (phase === 'core-finish') {
    // Resume only after core reached the final offline check; refuse other datasets.
    const novels = await invoke('get_all_novels')
    const books = (await invoke('get_all_vocab_books')).filter(book => !book.isPreset)
    const book = books.find(book => book.name === 'Licensed whole preset')
    const stats = await invoke('get_learning_stats')
    check(novels.length === 1 && books.length === 2 && book && stats.total_words === 3992 && stats.total_reviews === 1, 'Resume only the completed core workflow')
    const pdf = await readFile('test-data/native-license/export.pdf')
    check(pdf.subarray(0, 5).toString() === '%PDF-' && pdf.length > 1000, 'Completed core PDF')
    await checkpointOffline(novels[0].id, book.id)
    console.log(`CORE FINAL CHECK PASS: latest online baseline, fixed offline deadline, ${pdf.length}-byte real PDF and completed learning workflow`)
  } else if (phase === 'core') {
    await expect(page.getByRole('heading', { name: '激活词阅' })).toBeVisible()
    await page.screenshot({ path: 'test-data/native-license/activation-light.png', animations: 'disabled' })
    await rejection('get_all_novels', {}, 'ACTIVATION_REQUIRED')
    await invoke('backup_database', { destPath: backupPath })
    check((await readFile(backupPath)).subarray(0, 15).toString() === 'SQLite format 3', 'Unlicensed backup remains available')
    await page.getByRole('textbox', { name: '词阅卡密', exact: true }).fill(fixture.other.cardKey)
    await page.getByRole('button', { name: '激活', exact: true }).click()
    await expect(page.getByRole('alert')).toContainText('不适用于词阅')
    await page.getByRole('textbox', { name: '词阅卡密', exact: true }).fill(cardKey.toLowerCase())
    await page.getByRole('button', { name: '激活', exact: true }).click()
    await expect(page.locator('.novel-list-page')).toBeVisible()
    const license = await invoke('get_license_status')
    check(license.authorized && license.mode === 'online' && license.plan === '30d', 'Native activation')
    check(!JSON.stringify(license).includes(cardKey) && !('token' in license), 'Secrets stay in Rust')
    const encrypted = await readFile(resolve(info.dataDir, 'license/license.cache'))
    check(!encrypted.includes(Buffer.from(cardKey)), 'Windows encrypted card cache')
    console.log('Native activation, protected commands, product isolation and encrypted cache PASS')
    const otherDevice = await fetch('http://127.0.0.1:18100/api/v1/licenses/activate', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ card_key: cardKey, device_id: 'another-native-test-device-012345', client_version: '0.1.0' }) })
    check(otherDevice.status === 409 && (await otherDevice.json()).error.code === 'DEVICE_MISMATCH', 'Device binding enforced by real server')
    const imported = await invoke('import_preset_vocab_book', { presetKey: 'cet6-all', newBookName: 'Licensed whole preset', requestId: 'native-license' })
    check(imported.newWords === 3992, 'Licensed whole import')
    console.log('Native whole preset import PASS')
    const readingBook = await invoke('create_vocab_book', { name: 'Licensed reading', description: '' })
    const txtPath = resolve('test-data/native-license/novel.txt')
    await writeFile(txtPath, 'Licensed Novel\n\n第一章 奇迹\n\n' + Array.from({ length: 30 }, () => 'The world was full of wonder. Mary opened the door and saw a beautiful garden.').join('\n\n') + '\n\n第二章 清晨\n\nThe world was quiet.\n')
    await invoke('plugin:event|emit', { event: 'tauri://drag-drop', payload: { paths: [txtPath], position: { x: 200, y: 180 } } })
    await page.getByRole('button', { name: '确认并开始阅读', exact: true }).click({ timeout: 60000 })
    await expect(page.locator('.ProseMirror')).toHaveAttribute('contenteditable', 'false')
    const novelId = Number(new URL(page.url()).pathname.split('/').at(-1))
    await page.getByRole('button', { name: '学习工具', exact: true }).click()
    await page.locator('.reading-tools-form .el-select').first().click(); await page.getByRole('option', { name: /英文/ }).click()
    await page.locator('.reading-tools-form .el-select').last().click(); await page.getByRole('option', { name: 'Licensed reading', exact: true }).click()
    await page.getByRole('dialog', { name: '学习工具' }).getByRole('button', { name: '关闭', exact: true }).click()
    await page.locator('.ProseMirror p').first().evaluate(el => { const node = el.firstChild, offset = node.textContent.indexOf('wonder'), range = document.createRange(); range.setStart(node, offset); range.setEnd(node, offset + 6); const selection = window.getSelection(); selection.removeAllRanges(); selection.addRange(range) })
    await page.locator('.ProseMirror').dispatchEvent('mouseup', { clientX: 220, clientY: 180 })
    await expect(page.locator('.dict-word')).toHaveText('wonder')
    await page.getByRole('button', { name: '加入词汇本', exact: true }).click(); await expect(page.getByRole('button', { name: '已加入', exact: true })).toBeVisible()
    await page.keyboard.press('Escape')
    await page.goto(`http://localhost:1420/vocabulary/${readingBook.id}/review`)
    await expect(page.locator('.review-card')).toBeVisible(); await page.getByRole('button', { name: '显示答案', exact: true }).click(); await page.getByRole('button', { name: /掌握/ }).click()
    await expect(page.getByRole('heading', { name: '今日复习完成' })).toBeVisible()
    await invoke('backup_database', { destPath: backupPath }); await invoke('restore_database', { srcPath: backupPath })
    const outputPath = resolve('test-data/native-license/export.pdf')
    await invoke('export_pdf', { novelId, templateType: 'intensive', vocabBookId: imported.bookId, steps: [1, 2, 3], cover: false, pageNumbers: false, outputPath })
    const pdf = await readFile(outputPath); check(pdf.subarray(0, 5).toString() === '%PDF-' && pdf.length > 1000, 'Real Rust PDF')
    const invalid = cardKey.slice(0, -1) + (cardKey.endsWith('0') ? '1' : '0')
    await rejection('activate_license', { cardKey: invalid }, 'INVALID_CARD')
    check((await invoke('get_license_status')).authorized, 'Failed replacement keeps original license')
    await page.goto('http://localhost:1420/settings')
    await page.getByRole('tab', { name: '词阅授权' }).click()
    await expect(page.getByText('在线已验证', { exact: true })).toBeVisible()
    await page.screenshot({ path: 'test-data/native-license/settings-light.png', animations: 'disabled' })
    await checkpointOffline(novelId, imported.bookId)
    console.log(`CORE PASS: activation, product/device isolation, protected invoke, encrypted cache, whole import, reading lookup/collection, review, backup/restore, ${pdf.length}-byte PDF, replacement, fixed offline deadline`)
  } else {
    const saved = JSON.parse(await readFile(statePath, 'utf8'))
    const status = await invoke('verify_license')
    if (phase === 'restart') {
      check(status.authorized && status.mode === 'offline' && status.deviceId === saved.deviceId && status.expiresAt === saved.expiresAt && status.tokenExpiresAt === saved.tokenExpiresAt, 'Restart must not extend offline authorization')
      if (process.env.NATIVE_EXPECT_NETWORK_DOWN === '1') check(status.code === 'NETWORK_UNAVAILABLE', 'Actual disconnected API must use signed offline cache')
      check((await invoke('get_learning_stats')).total_reviews === 1, 'Learning data survives licensed restart')
      await expect(page.locator('.novel-list-page')).toBeVisible()
      console.log('RESTART PASS: real offline restart, encrypted cache, stable device and fixed deadlines, retained learning data')
    } else if (phase === 'lifecycle') {
      control('online'); check((await invoke('verify_license')).authorized, 'Online recovery')
      await page.goto(`http://localhost:1420/novels/${saved.novelId}?mode=edit`)
      await expect(page.locator('.ProseMirror')).toHaveAttribute('contenteditable', 'true')
      await page.locator('.ProseMirror').click(); await page.keyboard.press('Control+End'); await page.keyboard.type(' Pending licensed edit.')
      control('disable'); check((await invoke('verify_license')).code === 'LICENSE_DISABLED', 'Server disable locks')
      await rejection('get_all_novels', {}, 'LICENSE_DISABLED')
      await expect(page.getByRole('heading', { name: '激活词阅' })).toBeVisible()
      check(await page.locator('.ProseMirror').textContent().then(text => text.includes('Pending licensed edit.')), 'Lock preserves mounted editor text')
      await expect(page.locator('.save-status')).not.toContainText('未保存')
      await invoke('backup_database', { destPath: backupPath })
      control('outage'); check(!(await invoke('verify_license')).authorized, 'Denial prevents offline reuse')
      control('online'); control('enable'); check((await invoke('verify_license')).authorized, 'Admin enable permits retry')
      await page.getByRole('button', { name: '重新校验', exact: true }).click()
      await expect(page.getByRole('heading', { name: '激活词阅' })).toHaveCount(0)
      check((await invoke('get_chapters', { novelId: saved.novelId })).some(chapter => chapter.content.includes('Pending licensed edit.')), 'Pending editor save is preserved')
      control('unbind'); check((await invoke('verify_license')).code === 'ACTIVATION_REQUIRED', 'Unbind requires explicit activation')
      const rebound = await invoke('activate_license', { cardKey }); check(rebound.authorized && rebound.expiresAt === saved.expiresAt, 'Rebinding does not restart plan')
      control('shorten'); const short = await invoke('verify_license'); check(short.authorized && short.tokenExpiresAt === short.expiresAt, 'Finite plan caps offline token')
      await expect.poll(async () => (await invoke('get_license_status')).authorized, { timeout: 10000 }).toBe(false)
      await rejection('get_all_novels', {}, 'LICENSE_EXPIRED')
      await expect(page.getByRole('heading', { name: '激活词阅' })).toBeVisible({ timeout: 35000 })
      await page.screenshot({ path: 'test-data/native-license/expired-light.png', animations: 'disabled' })
      await page.getByRole('switch', { name: '主题' }).focus(); await page.keyboard.press('Space'); await page.getByRole('button', { name: '切换为 English' }).click()
      await expect(page.locator('html')).toHaveClass(/dark/)
      await page.screenshot({ path: 'test-data/native-license/expired-dark-en.png', animations: 'disabled' })
      await page.getByRole('textbox', { name: 'WordRead card key', exact: true }).fill(fixture.lifetime.cardKey)
      await page.getByRole('button', { name: 'Activate', exact: true }).click()
      await expect(page.getByRole('heading', { name: 'Activate WordRead' })).toHaveCount(0)
      const lifetime = await invoke('get_license_status'); check(lifetime.authorized && lifetime.plan === 'lifetime' && lifetime.expiresAt === null, 'Lifetime still uses capped signed token')
      check((await invoke('get_learning_stats')).total_reviews === 1, 'Expiry never deletes learning data')
      console.log('LIFECYCLE PASS: disable, backup while locked, no offline revival, enable, unbind/reactivate with original deadline, local expiry, dark/English gate, lifetime replacement, retained data')
    } else throw new Error('Unknown native test phase')
  }
} finally { await browser.close().catch(() => {}) }
