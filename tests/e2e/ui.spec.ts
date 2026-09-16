import { test, expect } from '@playwright/test'

async function bridge(page: import('@playwright/test').Page, options: { empty?:boolean; failed?:boolean; dark?:boolean; english?:boolean; multipleChapters?:boolean; largeChapter?:boolean; savedReading?:boolean; largeLists?:boolean; multipleBooks?:boolean } = {}) {
  await page.addInitScript((options) => {
    localStorage.setItem('theme', options.dark ? 'dark' : 'light')
    localStorage.setItem('app_locale', options.english ? 'en' : 'zh')
    const novels = options.empty ? [] : [{ id:1,title:'The Secret Garden',author:'Frances Hodgson Burnett',category:'小说',language:'en',isFavorite:false,rawText:'',cleanedText:'',createdAt:'2026-09-16',updatedAt:'2026-09-16' }]
    const books = options.empty ? [] : [{ id:1,name:'阅读生词',description:'Words from reading',isPreset:false,createdAt:'2026-09-16',updatedAt:'2026-09-16' }]
    if(options.multipleBooks)books.push({...books[0],id:2,name:'阅读词汇二'})
    const words = options.largeLists ? Array.from({length:1000}, (_,i) => ({id:i+1,vocabBookId:1,word:`garden-${i}`,definition:'A place where plants are grown.',phonetic:'/ˈɡɑːdn/',exampleSentence:'Mary opened the garden door.',novelId:1,chapterId:1,proficiency:'unknown',memoryTag:'',createdAt:'2026-09-16',matchTerms:''})) : []
    const personalWords=words.map((word,i)=>({...word,id:i+1,userVocabId:i+1,lastReviewedAt:0,active:i%2===0,sourceBooks:i%2===0 ? [{id:1,name:'阅读生词'}] : []}))
    const presets=Array.from({length:40},(_,i)=>({id:i+1,name:`考试合集与教材第 ${i+1} 册`,description:'离线词汇来源与完整释义',presetKey:i===0?'cet6-all':`fixture-${i}`,wordCount:i===0?3992:300,category:i<13?'university':'textbook',sources:[`Source_${i+1}`]}))
    if (options.largeLists) {
      novels[0].title = 'The Secret Garden: A Very Long Novel Title With Additional Notes for Language Learners '.repeat(3)
      books[0].name = '阅读词汇本与长期学习计划 '.repeat(8)
      novels.push(...Array.from({length:149}, (_,i) => ({...novels[0],id:i+2,title:`Novel ${i+2}`})))
      books.push(...Array.from({length:149}, (_,i) => ({...books[0],id:i+2,name:`Vocabulary ${i+2}`})))
    }
    const content = '<h1>Chapter One</h1>' + Array.from({length:80}, (_,i) => `<p>The garden was full of wonder. Mary opened the little door and found a beautiful world. Paragraph ${i}.</p>`).join('')
    const chapters = [{id:1,novelId:1,title:'Chapter One',sortOrder:0,startIndex:0,content,createdAt:'2026-09-16',updatedAt:'2026-09-16'}]
    if (options.multipleChapters) chapters.push({...chapters[0],id:2,title:'Chapter Two',sortOrder:1,startIndex:content.length,content:'<h1>Chapter Two</h1><p>Second chapter content.</p>'})
    if (options.largeChapter) chapters.push({...chapters[0],id:2,title:'Chapter Two',sortOrder:1,startIndex:content.length,content:'<h1>Chapter Two</h1>'+Array.from({length:2000},(_,i) => `<p>Large paragraph ${i}. The garden was full of wonder. Mary opened the little door and found a beautiful world.</p>`).join('')})
    const state = window as any
    state.testCalls = []; state.failRequests = options.failed; state.failExport = false
    state.__TAURI_INTERNALS__ = {
      metadata:{ currentWindow:{label:'main'},currentWebview:{label:'main'} },
      transformCallback:() => 1, unregisterCallback:() => {},
      invoke:async (cmd:string,args:any = {}) => {
        state.testCalls.push({cmd,args})
        if (state.failRequests && ['get_all_novels','get_all_vocab_books','get_due_words_count','get_learning_stats','get_all_due_words','list_preset_vocab_books'].includes(cmd)) throw new Error('Database unavailable')
        if (cmd === 'get_app_info') return {version:'0.1.0',dataDir:'C:/test',dbSize:0}
        if (cmd === 'get_all_settings') return [{key:'theme',value:options.dark ? 'dark' : 'light'}]
        if (cmd === 'get_setting') return options.savedReading && args.key === 'reading_pos_1' ? JSON.stringify({chapterIndex:1,percent:0.6}) : ''
        if (cmd === 'get_all_novels' || cmd === 'search_novels') return novels
        if (cmd === 'get_novel_meta' || cmd === 'get_novel') return novels[0]
        if (cmd === 'get_novel_content') return content
        if (cmd === 'get_chapter_list' || cmd === 'get_chapters') return chapters
        if (cmd === 'get_chapter_content') return chapters.find(ch => ch.id === args.chapterId)
        if (cmd === 'get_all_vocab_books') return books
        if (cmd === 'get_vocab_words') return words
        if (cmd === 'get_user_vocab_page') {
          if(state.failRequests) throw new Error('Database unavailable')
          const selected=personalWords.filter(w=>(!args.query || w.word.includes(args.query)) && (!args.proficiencies || args.proficiencies.includes(w.proficiency)))
          return {total:selected.length,words:selected.slice(args.offset,args.offset+args.limit)}
        }
        if (cmd === 'set_user_vocab_proficiency') {personalWords.filter(w=>args.ids.includes(w.id)).forEach(w=>w.proficiency=args.proficiency);return args.ids.length}
        if (cmd === 'list_preset_vocab_books') return presets
        if (cmd === 'import_preset_vocab_book') {if(state.failImport) throw new Error('Resource unavailable');return {bookId:1,imported:3992,newWords:3000,inherited:992,skipped:0}}
        if (cmd === 'get_highlight_words' || cmd === 'get_all_due_words' || cmd === 'get_due_words') return []
        if (cmd === 'get_vocab_words_page') return {total:words.length,words:words.slice(args.offset || 0,(args.offset || 0)+(args.limit || 50))}
        if (cmd === 'get_due_words_count') return 0
        if (cmd === 'get_review_progress') return { reviewed_today:0,goal:20,due_total:0 }
        if (cmd === 'get_learning_stats') return { total_words:0,total_books:books.length,total_novels:novels.length,by_proficiency:{unknown:0,familiar:0,mastered:0},reviewed_today:0,total_reviews:0,reviews_last_7_days:[] }
        if (cmd === 'get_ai_settings') return { enabled:false,provider:'openai',api_key:'',model:'',base_url:'' }
        if (cmd === 'plugin:dialog|save') return 'C:/test/garden.pdf'
        if (cmd === 'export_pdf') { if (state.failExport) throw new Error('Export unavailable'); return {path:'C:/test/garden.pdf',total_vocab:0,matched_words:0,chapter_count:1,steps_used:'1,2,3'} }
        if (cmd === 'plugin:event|listen') return 1
        if (cmd === 'dict_lookup_english') return {word:args.word,phonetic:'',definition:'花园',translations:[]}
        return null
      },
    }
    state.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener:() => {} }
  }, options)
}

test('desktop navigation is stable, collapsible and exposes review', async ({ page }) => {
  await bridge(page); await page.goto('/')
  await expect(page.getByRole('heading', {name:'今天，从一段阅读开始'})).toBeVisible()
  await expect(page.locator('.sidebar')).toBeVisible()
  await page.screenshot({path:'test-results/home-light.png',fullPage:true,animations:'disabled'})
  const before = await page.locator('.main-area').boundingBox()
  await page.locator('.sidebar').hover()
  expect((await page.locator('.main-area').boundingBox())?.x).toBe(before?.x)
  await page.getByRole('button', {name:'收起导航'}).click()
  await expect(page.locator('.sidebar')).toHaveClass(/collapsed/)
  await page.locator('.sidebar a[href="/review"]').click()
  await expect(page.locator('.sidebar a[href="/review"]')).toHaveAttribute('aria-current','page')
})

test('reading panels preserve scroll and editing deep links remain editable', async ({ page }) => {
  await bridge(page); await page.goto('/novels')
  await page.getByRole('link', {name:'The Secret Garden'}).click()
  await expect(page).toHaveURL(/mode=read/)
  await expect(page.locator('.ProseMirror')).toHaveAttribute('contenteditable','false')
  await page.screenshot({path:'test-results/reader-light.png',fullPage:true,animations:'disabled'})
  await page.locator('.tiptap-editor').evaluate(el => { el.scrollTop = 400 })
  const position = await page.locator('.tiptap-editor').evaluate(el => el.scrollTop)
  await page.getByRole('button', {name:'目录',exact:true}).click()
  await expect(page.getByRole('dialog', {name:'目录'})).toBeVisible()
  await page.keyboard.press('Escape')
  await page.getByRole('button', {name:'学习工具',exact:true}).click()
  await expect(page.getByRole('dialog', {name:'学习工具'})).toBeVisible()
  await page.keyboard.press('Escape')
  expect(await page.locator('.tiptap-editor').evaluate(el => el.scrollTop)).toBe(position)
  await page.getByRole('button', {name:'编辑正文'}).click()
  await expect(page.locator('.ProseMirror')).toHaveAttribute('contenteditable','true')
  await expect(page.locator('.sidebar')).toBeVisible()
  await expect(page.locator('.save-status')).not.toContainText('未保存')
})

test('PDF export config reports failure and supports retry with existing arguments', async ({ page }) => {
  await bridge(page); await page.goto('/novels/1?mode=read')
  await expect(page.locator('.ProseMirror')).toBeVisible()
  await page.getByRole('button', {name:'导出 PDF',exact:true}).click()
  await page.evaluate(() => { (window as any).failExport = true })
  await page.getByRole('button', {name:'开始导出'}).click()
  await expect(page.getByText('导出失败：Export unavailable')).toBeVisible()
  await page.evaluate(() => { (window as any).failExport = false })
  await page.getByRole('button', {name:'重试',exact:true}).click()
  await expect(page.getByText('导出完成：C:/test/garden.pdf')).toBeVisible()
  const args = await page.evaluate(() => (window as any).testCalls.filter((call:any) => call.cmd === 'export_pdf').at(-1).args)
  expect(args.novelId).toBe(1); expect(args.steps).toEqual([1,2,3]); expect(args.templateType).toBe('intensive')
})

test('empty home imports directly and failed home can recover', async ({ page }) => {
  await bridge(page, {empty:true, failed:true}); await page.goto('/')
  await expect(page.getByText('部分学习数据未能加载，请重试。')).toBeVisible()
  await expect(page.getByText('今天的复习已完成')).not.toBeVisible()
  await page.evaluate(() => { (window as any).failRequests = false })
  await page.getByRole('button', {name:'重试',exact:true}).click()
  await expect(page.getByRole('heading', {name:'开始你的第一本小说'})).toBeVisible()
  await page.getByRole('button', {name:'导入小说'}).first().click()
  await expect(page.getByRole('dialog', {name:'导入小说'})).toBeVisible()
})

test('narrow-screen navigation opens explicitly and closes after navigation', async ({ page }) => {
  await page.setViewportSize({width:390,height:844}); await bridge(page); await page.goto('/')
  await expect(page.locator('.sidebar')).not.toBeVisible()
  await page.getByRole('button', {name:'导航',exact:true}).click()
  await expect(page.getByRole('dialog', {name:'导航'})).toBeVisible()
  await page.locator('.sidebar a[href="/vocabulary"]').click()
  await expect(page).toHaveURL('/vocabulary')
  await expect(page.getByRole('dialog', {name:'导航'})).not.toBeVisible()
})

for (const options of [{dark:false,english:false},{dark:true,english:true},{dark:false,english:true},{dark:true,english:false}]) {
  for (const size of [{width:960,height:640},{width:800,height:500},{width:390,height:844}]) {
    test(`all pages fit ${size.width}x${size.height}, ${options.dark ? 'dark' : 'light'}, ${options.english ? 'English' : 'Chinese'}`, async ({page}) => {
      test.setTimeout(60000)
      const errors:string[] = []; page.on('pageerror', e => errors.push(e.message))
    await bridge(page, options)
      await page.setViewportSize(size)
      for (const [route, root] of [
        ['/', '.home-page'], ['/novels', '.novel-list-page'],
        ['/vocabulary', '.vocab-book-list-page'], ['/vocabulary/1', '.vocab-book-detail-page'],
        ['/vocabulary?tab=all', '.all-vocabulary-panel'],
        ['/presets', '.preset-page'], ['/stats', '.stats-page'],
        ['/settings', '.settings-page'], ['/review', '.review-page'],
        ['/novels/1?mode=read', '.editor-page'],
      ]) {
        await page.goto(route)
        await expect(page.locator(root)).toBeVisible()
        await expect(page.locator(`${root} .is-loading, ${root} [aria-busy="true"], ${root} .el-loading-mask`)).toHaveCount(0)
        if (route.includes('mode=read')) await expect(page.locator('.ProseMirror')).toBeVisible()
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
        expect(await page.locator('main').evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true)
        if (size.width === 960 && options.dark === options.english) {
          const name = route === '/' ? 'home' : route.includes('mode=read') ? 'reader' : route.includes('tab=all') ? 'all-vocabulary' : route === '/vocabulary/1' ? 'vocabulary-detail' : route.split('/')[1]
          await page.screenshot({path:`.ui-preview/${name}-${options.dark ? 'dark' : 'light'}.png`,animations:'disabled'})
        }
      }
      expect(errors).toEqual([])
    })
  }
}

test('personal registry supports pagination, shared batch marking, search and retry',async({page})=>{
  await bridge(page,{largeLists:true});await page.goto('/vocabulary?tab=all')
  await expect(page.locator('.el-table__body-wrapper tbody tr')).toHaveCount(50)
  await expect(page.getByText('复习暂停',{exact:true}).first()).toBeVisible()
  await page.getByRole('button',{name:'下一页',exact:true}).click()
  await expect(page.locator('.el-table__body-wrapper tbody tr').first()).toContainText('garden-50')
  await page.locator('.el-table__body-wrapper tbody tr').first().locator('.el-checkbox').click()
  await expect(page.locator('.el-table__body-wrapper tbody tr').first().getByRole('checkbox')).toBeChecked()
  await page.getByRole('button',{name:'标为已掌握',exact:true}).click()
  await expect.poll(()=>page.evaluate(()=>(window as any).testCalls.some((c:any)=>c.cmd==='set_user_vocab_proficiency'&&c.args.proficiency==='mastered'&&c.args.ids[0]===51))).toBe(true)
  await page.getByRole('textbox',{name:'搜索全部单词和释义'}).fill('garden-999')
  await expect(page.locator('.el-table__body-wrapper tbody tr')).toHaveCount(1)
  await page.evaluate(()=>{(window as any).failRequests=true})
  await page.getByRole('textbox',{name:'搜索全部单词和释义'}).fill('garden-998')
  await expect(page.getByText('Database unavailable',{exact:true})).toBeVisible()
  await page.evaluate(()=>{(window as any).failRequests=false})
  await page.getByRole('button',{name:'重试',exact:true}).click()
  await expect(page.locator('.el-table__body-wrapper tbody tr')).toContainText('garden-998')
})

test('forty preset entries import with failure retry and inherited-state feedback',async({page})=>{
  await bridge(page);await page.goto('/presets')
  await expect(page.locator('.preset-card')).toHaveCount(40)
  await page.evaluate(()=>{(window as any).failImport=true})
  await page.getByRole('button',{name:'整套导入',exact:true}).first().click()
  const dialog=page.getByRole('dialog',{name:'整套导入词表'})
  await dialog.getByRole('button',{name:'整套导入',exact:true}).click()
  await expect(dialog.getByText('Resource unavailable',{exact:true})).toBeVisible()
  await page.evaluate(()=>{(window as any).failImport=false})
  await dialog.getByRole('button',{name:'重试',exact:true}).click()
  await expect(dialog).toContainText('992')
  await dialog.getByRole('button',{name:'打开词汇本',exact:true}).click()
  await expect(page).toHaveURL('/vocabulary/1')
})

test('personal word status is fully reachable without horizontal scrolling at supported widths',async({page})=>{
  await bridge(page,{largeLists:true})
  for (const size of [{width:960,height:640},{width:800,height:500},{width:390,height:844}]) {
    await page.setViewportSize(size);await page.goto('/vocabulary?tab=all')
    await expect(page.locator('.el-table__body-wrapper tbody tr')).toHaveCount(50)
    const control=page.getByRole('combobox',{name:'garden-0的熟练度',exact:true})
    const bounds=await control.locator('..').boundingBox()
    expect(bounds).not.toBeNull()
    expect(bounds!.x).toBeGreaterThanOrEqual(0)
    expect(bounds!.x+bounds!.width).toBeLessThanOrEqual(size.width)
  }
})

test('multi-chapter reading keeps the selected chapter and usable mobile controls', async ({ page }) => {
  await bridge(page, {multipleChapters:true}); await page.goto('/novels/1?mode=read')
  await expect(page.locator('.ProseMirror')).toBeVisible()
  await page.getByRole('button',{name:'目录',exact:true}).click()
  await page.locator('.chapter-item').nth(1).click()
  await expect(page.locator('.ProseMirror')).toContainText('Second chapter content.')
  await expect(page.locator('.reading-progress-wrap')).toContainText('Chapter Two')
  await page.setViewportSize({width:390,height:844})
  await expect(page.getByRole('button',{name:/上一章/})).toBeVisible()
  await page.getByRole('button',{name:/上一章/}).click()
  await expect(page.locator('.ProseMirror')).toContainText('Chapter One')
})

test('reading keyboard shortcuts allow PDF checkbox interaction', async ({ page }) => {
  await bridge(page); await page.goto('/novels/1?mode=read')
  await expect(page.locator('.ProseMirror')).toBeVisible()
  await page.getByRole('button',{name:'导出 PDF',exact:true}).click()
  const cover = page.getByRole('dialog',{name:'导出 PDF'}).locator('.el-checkbox input').last()
  await cover.focus(); await page.keyboard.press('Space')
  await expect(cover).toBeChecked()
})

test('navigation footer stays reachable in the minimum desktop window', async ({ page }) => {
  await page.setViewportSize({width:800,height:500}); await bridge(page); await page.goto('/')
  const footer = await page.locator('.sidebar footer').boundingBox()
  expect(footer!.y + footer!.height).toBeLessThanOrEqual(500)
})

test('new novel deep link opens creation instead of an invalid editor', async ({ page }) => {
  await bridge(page); await page.goto('/novels/new')
  await expect(page.getByRole('dialog')).toBeVisible()
  await expect(page).toHaveURL('/novels')
  await expect(page.getByText('无法加载小说')).not.toBeVisible()
})

test('large chapter read-only loading does not create unsaved edits', async ({ page }) => {
  await bridge(page, {largeChapter:true,savedReading:true}); await page.goto('/novels/1?mode=read')
  await expect(page.locator('.ProseMirror')).toContainText('Large paragraph 1999', {timeout:15000})
  await page.getByRole('button',{name:'编辑正文',exact:true}).click()
  await expect(page.locator('.save-status')).not.toContainText('未保存')
})

test('large non-first chapter restores its saved local reading position', async ({ page }) => {
  await bridge(page, {largeChapter:true,savedReading:true}); await page.goto('/novels/1?mode=read')
  await expect(page.locator('.ProseMirror')).toContainText('Large paragraph 1999', {timeout:15000})
  await expect.poll(() => page.locator('.tiptap-editor').evaluate(el => el.scrollTop / (el.scrollHeight - el.clientHeight))).toBeCloseTo(0.6, 1)
  await expect(page.locator('.reading-progress-wrap')).toContainText('Chapter Two')
})

test('long titles and large lists keep primary actions reachable', async ({ page }) => {
  test.setTimeout(60000)
  await bridge(page, {largeLists:true})
  for (const size of [{width:960,height:640},{width:800,height:500},{width:390,height:844}]) {
    await page.setViewportSize(size)
    for (const [route, root] of [['/novels','.novel-list-page'],['/vocabulary','.vocab-book-list-page'],['/vocabulary/1','.vocab-book-detail-page'],['/novels/1?mode=read','.editor-page']]) {
      await page.goto(route); await expect(page.locator(root)).toBeVisible()
      await expect(page.locator(`${root} .el-loading-mask`)).toHaveCount(0)
      if (route.includes('mode=read')) await expect(page.locator('.ProseMirror')).toBeVisible()
      expect(await page.locator('main').evaluate(el => el.scrollWidth <= el.clientWidth), `${route} at ${size.width}px`).toBe(true)
      const action = route === '/novels' ? page.getByRole('button',{name:'导入文件',exact:true}) : route === '/vocabulary' ? page.getByRole('button',{name:'新建词汇本',exact:true}) : route === '/vocabulary/1' ? page.getByRole('button',{name:'添加单词',exact:true}) : page.getByRole('button',{name:'编辑正文',exact:true})
      await action.scrollIntoViewIfNeeded(); await expect(action).toBeInViewport()
      if (size.width === 390) await page.screenshot({path:`.ui-preview/large-${root.slice(1)}-narrow.png`,animations:'disabled'})
    }
  }
})

test('dark theme selected settings retain readable text and keyboard focus', async ({ page }) => {
  await bridge(page, {dark:true}); await page.goto('/settings')
  await expect(page.locator('.settings-form').first()).toBeVisible()
  await expect(page.locator('.topbar .el-switch')).toBeVisible()
  await expect(page.getByRole('switch',{name:'主题',exact:true})).toHaveAccessibleName('主题')
  const selected = page.locator('.el-radio-button.is-active .el-radio-button__inner').first()
  const contrast = await selected.evaluate(el => {
    const style = getComputedStyle(el)
    const luminance = (color:string) => {
      const [r,g,b] = color.match(/[\d.]+/g)!.slice(0,3).map(n => {const c=Number(n)/255; return c<=0.04045 ? c/12.92 : ((c+0.055)/1.055)**2.4})
      return 0.2126*r+0.7152*g+0.0722*b
    }
    const fg=luminance(style.color),bg=luminance(style.backgroundColor)
    return (Math.max(fg,bg)+0.05)/(Math.min(fg,bg)+0.05)
  })
  expect(contrast).toBeGreaterThanOrEqual(4.5)
  const light = page.getByRole('radio',{name:'浅色',exact:true})
  await light.focus(); await page.keyboard.press('Space'); await expect(light).toBeChecked()
  const importLink = page.locator('.sidebar a[href="/novels"]')
  await importLink.focus()
  expect(await importLink.evaluate(el => getComputedStyle(el).outlineStyle)).toBe('solid')
  await page.keyboard.press('Enter'); await expect(page.locator('.novel-list-page')).toBeVisible()
})

test('table delete links remain readable in both themes', async ({ page }) => {
  for (const dark of [false,true]) {
    await bridge(page, {largeLists:true,dark}); await page.goto('/vocabulary/1')
    const action = page.locator('.el-table button.el-button--danger').first()
    await expect(action).toBeVisible()
    const contrast = await action.evaluate(el => {
      const text=getComputedStyle(el).color
      const probe=document.createElement('span'); probe.style.color='var(--bg-primary)'; el.append(probe)
      const background=getComputedStyle(probe).color; probe.remove()
      const luminance=(color:string) => {
        const [r,g,b]=color.match(/[\d.]+/g)!.slice(0,3).map(n=>{const c=Number(n)/255;return c<=0.04045?c/12.92:((c+0.055)/1.055)**2.4})
        return .2126*r+.7152*g+.0722*b
      }
      const fg=luminance(text),bg=luminance(background)
      return (Math.max(fg,bg)+.05)/(Math.min(fg,bg)+.05)
    })
    expect(contrast).toBeGreaterThanOrEqual(4.5)
  }
})

test('file import can open the native picker with the keyboard', async ({ page }) => {
  await bridge(page, {empty:true}); await page.goto('/novels?import=1')
  const picker = page.getByRole('button',{name:/点击选择/})
  await expect(picker).toBeVisible()
  await picker.focus(); await page.keyboard.press('Enter')
  await expect.poll(() => page.evaluate(() => (window as any).testCalls.some((call:any) => call.cmd === 'plugin:dialog|open'))).toBe(true)
})

test('PDF export main action fits the minimum window and narrow screen', async ({page}) => {
  await bridge(page)
  for (const size of [{width:800,height:500},{width:390,height:844}]) {
    await page.setViewportSize(size); await page.goto('/novels/1?mode=read')
    await expect(page.locator('.ProseMirror')).toBeVisible()
    await page.getByRole('button',{name:'导出 PDF',exact:true}).click()
    const start=page.getByRole('button',{name:'开始导出',exact:true})
    await expect(start).toBeVisible()
    const bounds=await start.boundingBox()
    expect(bounds!.y+bounds!.height).toBeLessThanOrEqual(size.height)
    await page.screenshot({path:`.ui-preview/export-${size.width}.png`,animations:'disabled'})
  }
})

test('dictionary book dropdown remains open for cross-book collection',async({page})=>{
  await bridge(page,{multipleBooks:true});await page.goto('/novels/1?mode=read')
  await expect(page.locator('.ProseMirror')).toBeVisible()
  await page.getByRole('button',{name:'学习工具',exact:true}).click()
  await page.locator('.reading-tools-form .el-select').first().click();await page.getByRole('option',{name:/英文/}).click()
  await page.getByRole('dialog',{name:'学习工具'}).getByRole('button',{name:'关闭',exact:true}).click()
  await page.locator('.ProseMirror p').first().evaluate(el=>{
    const node=el.firstChild!,offset=node.textContent!.indexOf('garden'),range=document.createRange()
    range.setStart(node,offset);range.setEnd(node,offset+6);const selection=window.getSelection()!;selection.removeAllRanges();selection.addRange(range)
  })
  await page.locator('.ProseMirror').dispatchEvent('mouseup',{clientX:220,clientY:180})
  const lookup=page.locator('.dict-lookup-popover');await expect(lookup.locator('.dict-word')).toHaveText('garden')
  await lookup.locator('.el-select__wrapper').click();await page.getByRole('option',{name:'阅读词汇二',exact:true}).click()
  await expect(lookup).toBeVisible()
  await lookup.getByRole('button',{name:'加入词汇本',exact:true}).click();await expect(lookup.getByRole('button',{name:'已加入',exact:true})).toBeVisible()
  await lookup.locator('.el-select__wrapper').click();await page.getByRole('option',{name:'阅读生词',exact:true}).click()
  await expect(lookup.getByRole('button',{name:'加入词汇本',exact:true})).toBeEnabled()
})
