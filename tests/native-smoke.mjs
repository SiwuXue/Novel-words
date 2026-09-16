// Run against an isolated Tauri dev instance with remote debugging on port 9229.
// Default: native drag-drop and real Rust PDF generation. NATIVE_PICKERS=1 also checks file dialogs with manual UI automation.
import { chromium, expect } from '@playwright/test'
import { mkdir, writeFile, readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
process.env.NO_PROXY = 'localhost,127.0.0.1'
await mkdir('test-results', {recursive:true})
const filePath = resolve('test-results/native-novel.txt')
const outputPath = resolve('test-results/native-export.pdf')
const usePickers = process.env.NATIVE_PICKERS === '1'
await writeFile(filePath, 'UI Smoke Novel\n\n第一章 花园\n\n'+Array.from({length:40}, () => 'The garden was full of wonder. Mary opened the door and saw a beautiful world.').join('\n\n')+'\n\n第二章 新的一天\n\nThe garden was quiet in the morning.\n', 'utf8')
const browser = await chromium.connectOverCDP('http://127.0.0.1:9229')
const page = browser.contexts()[0].pages()[0]
page.setDefaultTimeout(15000)
const invoke = (cmd,args={}) => page.evaluate(({cmd,args}) => window.__TAURI_INTERNALS__.invoke(cmd,args), {cmd,args})
const info = await invoke('get_app_info')
if (!info.dataDir.endsWith('com.tauri-app.novel-words-ui-test')) throw new Error('Native smoke requires isolated test identifier')
console.log('Isolated Tauri data:',info.dataDir)
const originalNovels = await invoke('get_all_novels')
const book = await invoke('create_vocab_book',{name:'UI Smoke Vocabulary',description:'Temporary verification fixture'})
let createdNovel
try {
  await page.goto('http://localhost:1420/novels')
  await expect(page.locator('.novel-list-page')).toBeVisible()
  await page.waitForTimeout(200)
  if (usePickers) {
    await page.getByRole('button',{name:'导入文件',exact:true}).click()
    await page.locator('.drop-zone').click()
    console.log('NATIVE_IMPORT_PICKER:', filePath)
    await expect(page.locator('.selected-file')).toBeVisible({timeout:180000})
    await page.getByRole('button',{name:/分析文件/}).click()
  } else {
    await invoke('plugin:event|emit',{event:'tauri://drag-drop',payload:{paths:[filePath],position:{x:200,y:180}}})
  }
  await page.getByRole('button',{name:'确认并开始阅读',exact:true}).click({timeout:30000})
  await expect(page.locator('.ProseMirror')).toBeVisible()
  const novelId = Number(new URL(page.url()).pathname.split('/').at(-1))
  createdNovel = novelId
  console.log('Native import: novel',novelId)
  if (!(await page.url()).includes('mode=read')) await page.getByRole('button',{name:'阅读模式'}).click()
  await expect(page.locator('.ProseMirror')).toHaveAttribute('contenteditable','false')
  await page.getByRole('button',{name:'学习工具',exact:true}).click()
  await page.locator('.reading-tools-form .el-select').first().click()
  await page.getByRole('option',{name:/英文/}).click()
  await page.locator('.reading-tools-form .el-select').last().click()
  await page.getByRole('option',{name:'UI Smoke Vocabulary',exact:true}).click()
  await page.getByRole('dialog',{name:'学习工具'}).getByRole('button',{name:'关闭',exact:true}).click()
  await page.locator('.ProseMirror p').first().evaluate(el => {
    const node = el.firstChild; const text = node.textContent; const offset = text.indexOf('garden')
    const range = document.createRange(); range.setStart(node,offset); range.setEnd(node,offset+6)
    const selection = window.getSelection(); selection.removeAllRanges(); selection.addRange(range)
  })
  await page.locator('.ProseMirror').dispatchEvent('mouseup',{clientX:220,clientY:180})
  await expect(page.locator('.dict-word')).toHaveText('garden')
  await page.getByRole('button',{name:'加入词汇本',exact:true}).click()
  await expect(page.getByRole('button',{name:'已加入',exact:true})).toBeVisible()
  const words = await invoke('get_vocab_words',{vocabBookId:book.id})
  if (!words.some(w => w.word === 'garden')) throw new Error('Collected word missing from SQLite')
  await page.keyboard.press('Escape')
  console.log('Native dictionary and collect: garden saved')
  if (usePickers) {
    await page.getByRole('button',{name:'导出 PDF',exact:true}).click()
    await page.getByRole('button',{name:'开始导出',exact:true}).click()
    console.log('NATIVE_PDF_PICKER:', outputPath)
    await expect(page.getByText('导出完成：'+outputPath)).toBeVisible({timeout:180000})
  } else {
    const exported = await invoke('export_pdf',{novelId,templateType:'intensive',vocabBookId:book.id,steps:[1,2,3],cover:false,pageNumbers:false,outputPath})
    if (exported.path !== outputPath) throw new Error('Unexpected PDF path')
  }
  const pdf = await readFile(outputPath)
  if (pdf.subarray(0,5).toString() !== '%PDF-' || pdf.length < 1000) throw new Error('Invalid native PDF output')
  console.log('Native PDF:',pdf.length,'bytes')
  if (usePickers) await page.getByRole('dialog',{name:'导出 PDF'}).getByRole('button',{name:'关闭',exact:true}).click()
  await page.screenshot({path:'test-results/native-reader.png',animations:'disabled'})
  await page.getByRole('button',{name:'编辑正文',exact:true}).click()
  await page.locator('.ProseMirror').click()
  await page.keyboard.press('Control+End'); await page.keyboard.type(' Native edit verification.')
  await page.keyboard.press('Control+s')
  await expect(page.locator('.save-status')).not.toContainText('未保存')
  const chapters = await invoke('get_chapters',{novelId})
  if (!chapters.some(ch => ch.content.includes('Native edit verification.'))) throw new Error('Edit not saved to SQLite')
  console.log('Native edit and manual save passed')
  await page.getByRole('button',{name:'返回列表',exact:true}).click()
  await page.locator('.sidebar a[href="/review"]').click()
  await expect(page.locator('.review-card')).toBeVisible()
  await page.getByRole('button',{name:'显示答案',exact:true}).click()
  await page.getByRole('button',{name:/掌握/}).click()
  await expect(page.getByRole('heading',{name:'今日复习完成'})).toBeVisible()
  const reviewed = await invoke('get_vocab_words',{vocabBookId:book.id})
  if (!reviewed[0].memoryTag) throw new Error('Review state not saved')
  console.log('Native review persisted')
} finally {
  if (!createdNovel) createdNovel = (await invoke('get_all_novels')).find(n => !originalNovels.some(old => old.id === n.id))?.id
  if (createdNovel) await invoke('delete_novel',{id:createdNovel})
  await invoke('delete_vocab_book',{id:book.id})
  await browser.close()
}
