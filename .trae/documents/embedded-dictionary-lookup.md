# 内嵌词典 + 划词查词 + 加入词汇本

## 概述

为词阅添加一个内嵌的本地词典，支持双向查词：
- **英→中**：双击/选中英文单词，弹出浮窗显示音标和中文释义
- **中→英**：选中中文词，弹出浮窗显示所有中文释义含该词的英文单词列表

两种查询都提供"加入词汇本"按钮一键收录生词。

## 现状分析

### 已具备的基础（来自 Phase 1 探索）

1. **词典数据已就位** — `DictionaryData/` 目录下：
   - `word.csv` — 7 万单词（带 uk/us 音标、词频、难度），`>` 分隔
   - `word_translation.csv` — 10 万中文翻译，标准 CSV（带引号转义）
   - `book.csv` + `relation_book_word.csv` — 词书关系（共 116MB，**本次不需要**）
2. **后端依赖齐备** — [Cargo.toml](file:///e:/Dsektop/tauri/novel-words/src-tauri/Cargo.toml#L20-L32) 已含 `rusqlite 0.31` (bundled)、`csv 1.3`、`serde 1`，可直接打开第二个 SQLite 连接
3. **现有 `vocab_word` 模型契合** — [vocabWord.ts](file:///e:/Dsektop/tauri/novel-words/src/types/vocabWord.ts#L1-L29) 的 `word/definition/phonetic` 字段正好对应词典的 `vc_vocabulary` / `translation` / `vc_phonetic_us`
4. **`create_vocab_word` 命令可复用** — [vocab_word.rs](file:///e:/Dsektop/tauri/novel-words/src-tauri/src/commands/vocab_word.rs#L7-L37) 已支持按 `vocab_book_id` 创建词条并去重
5. **Tiptap 编辑器可挂双击事件** — [NovelEditor.vue](file:///e:/Dsektop/tauri/novel-words/src/components/novel/NovelEditor.vue#L121-L136) 的 `editorProps` 已用于 paste 处理，DOM 双击事件可在 `editor-content` 元素上监听
6. **Tauri bundle 未配置 resources** — [tauri.conf.json](file:///e:/Dsektop/tauri/novel-words/src-tauri/tauri.conf.json#L28-L38) 的 `bundle` 字段目前只有 `icon`，需要新增 `resources`

### 关键决策

- **双向查词** — 双击英文单词查中译（精确匹配）；选中中文词查对应英文单词（LIKE 模糊匹配 translation 字段，返回多条结果）
- **只做"查词"，不做"词书浏览/导入词书"** — 用户原话明确：双击查词 + 加入词汇本。`book.csv` 和 `relation_book_word.csv`（116MB）本次完全不用，避免过度工程化
- **数据合并** — 把 `word.csv` 和 `word_translation.csv` 预处理成一个只读 `dictionary.db`（约 8-10MB），打包进安装包，开箱即用
- **独立只读连接** — 词典库与主库 `novel_words.db` 完全隔离，避免污染用户数据
- **浮动 popover 而非居中 dialog** — 查词是轻量阅读流的一部分，popover 出现在选中位置附近，点外部即关，不打断阅读
- **中文查询结果列表化** — 一个中文词可能对应多个英文单词（如"脑"可能对应 brain / mind / head），浮窗显示为列表，每条都可独立加入词汇本

## 实施步骤

### 步骤 1：数据预处理脚本（一次性）

**新建文件**：`scripts/build_dictionary_db.py`

读取 `DictionaryData/word.csv` 和 `word_translation.csv`，生成 `src-tauri/resources/dictionary.db`。

```sql
CREATE TABLE dict_word (
    word TEXT PRIMARY KEY,        -- 规范化小写形式，便于查询
    original TEXT,                -- 原始词形（保留大小写）
    phonetic_uk TEXT,
    phonetic_us TEXT,
    frequency REAL,
    difficulty INTEGER,
    translation TEXT              -- 中文释义（来自 word_translation.csv）
);
CREATE INDEX idx_dict_word_lookup ON dict_word(word);
```

处理要点：
- **去重策略**：以 `word_translation.csv` 的 `word` 字段为主表（含 10 万条释义），LEFT JOIN `word.csv` 的 `vc_vocabulary` 字段补充音标/词频。`word.csv` 中没匹配上的词条，音标留空
- **大小写**：建表时 `word` 字段存原始形式，但额外建一个 `word_lower` 生成列或直接用 `COLLATE NOCASE` 索引，实现大小写不敏感查询
- **批量插入**：用事务，每 5000 行 commit 一次，避免逐行插入慢
- **CSV 解析**：`word_translation.csv` 是标准 CSV（含逗号转义），必须用 `csv.reader` 不能 `split(',')`；`word.csv` 是 `>` 分隔
- 生成后文件大小预估 8-12MB（含索引）

### 步骤 2：Tauri 资源配置

**修改文件**：[src-tauri/tauri.conf.json](file:///e:/Dsektop/tauri/novel-words/src-tauri/tauri.conf.json#L28-L38)

在 `bundle` 中新增 `resources`：

```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [...],
  "resources": ["resources/dictionary.db"]
}
```

打包后 `dictionary.db` 会被复制到安装目录的 resources 子目录，运行时通过 `app.path().resource_dir()` 解析。

### 步骤 3：后端词典模块

**新建文件**：
- `src-tauri/src/dictionary.rs` — 词典连接管理 + 查询命令
- `src-tauri/src/models/dict_word.rs` — 响应结构体

**修改文件**：
- [src-tauri/src/lib.rs](file:///e:/Dsektop/tauri/novel-words/src-tauri/src/lib.rs) — 注册新模块、managed state、命令
- `src-tauri/src/models/mod.rs` — 导出 DictWord

#### 数据结构

```rust
// src-tauri/src/models/dict_word.rs
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct DictWord {
    pub word: String,
    pub phonetic_uk: String,
    pub phonetic_us: String,
    pub translation: String,
    pub frequency: f64,
    pub difficulty: i64,
}
```

#### 词典状态

```rust
// src-tauri/src/dictionary.rs
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

pub struct DictDbState {
    pub db: Mutex<Connection>,
}

impl DictDbState {
    pub fn open(resource_path: std::path::PathBuf) -> Result<Self, String> {
        let conn = Connection::open_with_flags(
            &resource_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).map_err(|e| format!("无法打开词典库: {}", e))?;
        Ok(Self { db: Mutex::new(conn) })
    }
}
```

#### 查询命令

支持双向查词，由前端根据选中文本自动选择调用哪个命令。

```rust
/// 英→中：精确查询单个英文单词（大小写不敏感）
#[tauri::command]
pub fn dict_lookup_english(
    state: State<DictDbState>,
    word: String,
) -> Result<Option<DictWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db
        .prepare(
            "SELECT word, phonetic_uk, phonetic_us, translation,
                    COALESCE(frequency, 0.0), COALESCE(difficulty, 0)
             FROM dict_word WHERE word = ?1 COLLATE NOCASE LIMIT 1",
        )?
        .query_row(rusqlite::params![word.trim()], |row| {
            Ok(DictWord {
                word: row.get(0)?,
                phonetic_uk: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                phonetic_us: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                translation: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                frequency: row.get(4)?,
                difficulty: row.get(5)?,
            })
        })
        .ok();
    Ok(result)
}

/// 中→英：模糊查询中文释义包含关键词的所有英文单词
/// 返回最多 20 条，按词频降序（高频词优先）
#[tauri::command]
pub fn dict_lookup_chinese(
    state: State<DictDbState>,
    keyword: String,
) -> Result<Vec<DictWord>, String> {
    let kw = keyword.trim();
    if kw.is_empty() {
        return Ok(Vec::new());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // 使用 LIKE %keyword% 匹配 translation 字段
    // 注意：SQLite LIKE 默认大小写不敏感（仅对 ASCII），中文不受影响
    let pattern = format!("%{}%", kw);
    let mut stmt = db
        .prepare(
            "SELECT word, phonetic_uk, phonetic_us, translation,
                    COALESCE(frequency, 0.0), COALESCE(difficulty, 0)
             FROM dict_word
             WHERE translation LIKE ?1 AND translation != ''
             ORDER BY frequency DESC
             LIMIT 20",
        )
        .map_err(|e| format!("查询失败: {}", e))?;
    let rows = stmt
        .query_map(rusqlite::params![pattern], |row| {
            Ok(DictWord {
                word: row.get(0)?,
                phonetic_uk: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                phonetic_us: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                translation: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                frequency: row.get(4)?,
                difficulty: row.get(5)?,
            })
        })
        .map_err(|e| format!("查询失败: {}", e))?;
    let results: Vec<DictWord> = rows.filter_map(|r| r.ok()).collect();
    Ok(results)
}
```

**性能说明**：
- 英文查询走主键索引，毫秒级
- 中文查询用 LIKE '%keyword%'，无法用索引，全表扫描 7 万行约 20-50ms（可接受）
- 如果后续性能不够，可改用 SQLite FTS5 全文索引（本期不做）

#### lib.rs 集成

在 `setup` 中：
```rust
let resource_path = app
    .path()
    .resource_dir()
    .map_err(|e| format!("无法解析资源目录: {}", e))?
    .join("resources")
    .join("dictionary.db");

let dict_state = DictDbState::open(resource_path)
    .map_err(|e| format!("词典库初始化失败: {}", e))?;
app.manage(dict_state);
```

在 `invoke_handler` 中新增 `dict_lookup_english` 和 `dict_lookup_chinese`。
**注意**：词典库加载失败**不**应该让应用退出（用户可能只想用基础功能），改为打日志 + 让 `dict_lookup_*` 命令返回错误。

### 步骤 4：前端词典状态

**新建文件**：`src/stores/dictionaryStore.ts`

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface DictWord {
  word: string
  phoneticUk: string
  phoneticUs: string
  translation: string
  frequency: number
  difficulty: number
}

/** 查询方向 */
export type LookupDirection = 'english' | 'chinese'

export const useDictionaryStore = defineStore('dictionary', () => {
  const looking = ref(false)
  // 英→中：单个结果（可能 null）
  const currentWord = ref<DictWord | null>(null)
  // 中→英：多条结果列表
  const chineseResults = ref<DictWord[]>([])
  const lookupError = ref('')
  // 当前查询方向，UI 用以决定渲染单条还是列表
  const direction = ref<LookupDirection>('english')

  /** 英→中查询 */
  async function lookupEnglish(word: string) {
    looking.value = true
    lookupError.value = ''
    direction.value = 'english'
    currentWord.value = null
    chineseResults.value = []
    try {
      currentWord.value = await invoke<DictWord | null>('dict_lookup_english', { word })
    } catch (e: any) {
      lookupError.value = String(e?.message || e)
    } finally {
      looking.value = false
    }
  }

  /** 中→英查询 */
  async function lookupChinese(keyword: string) {
    looking.value = true
    lookupError.value = ''
    direction.value = 'chinese'
    currentWord.value = null
    chineseResults.value = []
    try {
      chineseResults.value = await invoke<DictWord[]>('dict_lookup_chinese', { keyword })
    } catch (e: any) {
      lookupError.value = String(e?.message || e)
    } finally {
      looking.value = false
    }
  }

  /** 根据选中文本自动判断方向并查询 */
  async function lookupAuto(text: string) {
    const trimmed = text.trim()
    if (!trimmed) return
    // 含中文字符 → 中→英查询
    if (/[\u4e00-\u9fa5]/.test(trimmed)) {
      await lookupChinese(trimmed)
    } else {
      await lookupEnglish(trimmed)
    }
  }

  function clear() {
    currentWord.value = null
    chineseResults.value = []
    lookupError.value = ''
  }

  return {
    looking,
    currentWord,
    chineseResults,
    lookupError,
    direction,
    lookupEnglish,
    lookupChinese,
    lookupAuto,
    clear,
  }
})
```

### 步骤 5：词典浮窗组件

**新建文件**：`src/components/novel/DictLookupPopover.vue`

一个绝对定位的浮窗组件，根据传入的 `position`（屏幕坐标）和 `text` 显示。
支持两种渲染模式（由 store.direction 决定）：

**英→中模式（direction === 'english'）**：
- 顶部：英文单词 + 音标（按 speech_accent 偏好优先 us/uk）
- 中部：中文释义（多行）
- 底部：词汇本下拉选择 + "加入词汇本"按钮
- 词典无此词时显示"词典中无此词"

**中→英模式（direction === 'chinese'）**：
- 顶部：中文关键词 + "找到 N 个匹配单词"
- 中部：滚动列表，每条显示 [英文单词 + 音标 + 释义]，每条右侧有独立"加入"按钮
- 列表为空时显示"未找到对应英文单词"
- 底部：词汇本下拉选择（所有条目共用同一个目标词汇本）

加入词汇本调用现有 `create_vocab_word` 命令：
```typescript
await invoke('create_vocab_word', {
  vocabBookId: selectedBookId.value,
  word: dictWord.word,
  definition: dictWord.translation,
  phonetic: dictWord.phoneticUs || dictWord.phoneticUk,
  exampleSentence: '',
  novelId: props.novelId,
  proficiency: 'unknown',
  memoryTag: '',
})
```

成功后 toast 提示 + 对应条目按钮变为"已加入"。
失败（如单词已存在）显示错误 toast，但允许用户继续操作其他条目。

### 步骤 6：编辑器集成双击/选中事件

**修改文件**：[src/components/novel/NovelEditor.vue](file:///e:/Dsektop/tauri/novel-words/src/components/novel/NovelEditor.vue)

在 `editor-content` 元素上同时挂 `dblclick`（用于双击英文单词）和 `mouseup`（用于鼠标划选后释放）。

**双击（dblclick）**：
- 浏览器双击会自动选中一个词，用 `window.getSelection()` 拿到
- 校验为英文单词（`/^[A-Za-z][A-Za-z'-]*$/`）→ 调 `lookupEnglish`

**划选（mouseup）**：
- 用户按住左键拖动选中一段文本后释放
- 校验选中文本：
  - 纯英文单词（无空格）→ `lookupEnglish`
  - 含中文字符 → `lookupChinese`（取整个选中片段作为关键词，去除首尾空格）
  - 含空格的英文短语 → 忽略（避免误触发）
- 防抖：mouseup 后延迟 50ms 检查 selection，避免与 dblclick 冲突
- 若 selection 为空（用户只是点击没选），不弹窗

```typescript
import { ref, onMounted, onBeforeUnmount } from 'vue'
import DictLookupPopover from './DictLookupPopover.vue'
import { useDictionaryStore } from '@/stores/dictionaryStore'

const dictStore = useDictionaryStore()
const editorContentRef = ref<InstanceType<typeof EditorContent> | null>(null)
const popoverVisible = ref(false)
const popoverPosition = ref({ x: 0, y: 0 })
const popoverText = ref('')
let mouseupTimer: number | null = null

function getSelectionText(): string | null {
  const sel = window.getSelection()
  if (!sel || sel.isCollapsed) return null
  const text = sel.toString().trim()
  return text || null
}

function isEnglishWord(s: string): boolean {
  return /^[A-Za-z][A-Za-z'-]*$/.test(s)
}

function hasChinese(s: string): boolean {
  return /[\u4e00-\u9fa5]/.test(s)
}

function showPopover(x: number, y: number, text: string) {
  popoverText.value = text
  popoverPosition.value = { x, y }
  popoverVisible.value = true
  void dictStore.lookupAuto(text)
}

function handleDblClick(event: MouseEvent) {
  const text = getSelectionText()
  if (!text) return
  // 双击只处理英文单词
  if (!isEnglishWord(text)) return
  showPopover(event.clientX, event.clientY, text)
}

function handleMouseup(event: MouseEvent) {
  // 延迟 50ms 让 selection 同步稳定
  if (mouseupTimer) clearTimeout(mouseupTimer)
  mouseupTimer = window.setTimeout(() => {
    const text = getSelectionText()
    if (!text) return
    // 英文单词：交给 dblclick 处理（避免双击时重复弹窗）
    if (isEnglishWord(text) && text.length <= 50) {
      // 如果是单纯的英文单词双击场景，dblclick 会处理；这里只处理划选的非单词场景
      // 但划选一个英文单词（非双击触发）也应该弹窗，所以也处理
      showPopover(event.clientX, event.clientY, text)
      return
    }
    // 中文：划选后查中→英
    if (hasChinese(text) && text.length <= 30) {
      showPopover(event.clientX, event.clientY, text)
    }
    // 含空格的英文短语：忽略
  }, 50)
}

function closePopover() {
  popoverVisible.value = false
  dictStore.clear()
}

function onExternalClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.dict-lookup-popover')) {
    closePopover()
  }
}

onMounted(() => {
  const el = (editorContentRef.value as any)?.$el as HTMLElement | undefined
  el?.addEventListener('dblclick', handleDblClick)
  el?.addEventListener('mouseup', handleMouseup)
  document.addEventListener('mousedown', onExternalClick)
})
onBeforeUnmount(() => {
  const el = (editorContentRef.value as any)?.$el as HTMLElement | undefined
  el?.removeEventListener('dblclick', handleDblClick)
  el?.removeEventListener('mouseup', handleMouseup)
  document.removeEventListener('mousedown', onExternalClick)
  if (mouseupTimer) clearTimeout(mouseupTimer)
})
```

模板里加：
```vue
<editor-content ref="editorContentRef" :editor="editor" class="tiptap-editor" />
<DictLookupPopover
  v-if="popoverVisible"
  :text="popoverText"
  :position="popoverPosition"
  :novel-id="props.novelId"
  @close="closePopover"
/>
```

**关键点**：
- 双击英文单词时，浏览器先触发 dblclick，selection 已是单词 → 直查
- 鼠标划选中文时，用户按住拖动 → 释放时 mouseup → 延迟 50ms 拿到完整 selection → 查中→英
- 划选英文单词（非双击）也支持
- 含空格的英文短语忽略（如 "pick up"），避免误触发；后续若要支持短语查询再扩展
- 点击 popover 外部区域关闭

### 步骤 7：词典库的语音偏好

应用设置中已有 `speech_accent`（'us'/'uk'），浮窗显示音标时优先使用该偏好对应的音标，避免词典与已有功能不一致。

### 步骤 8：词典数据预处理脚本执行

运行 Python 脚本生成 `dictionary.db`，验证：
1. 文件大小在 5-15MB 之间
2. 用 SQLite CLI 抽查：`SELECT word, translation FROM dict_word WHERE word='brain' LIMIT 1;`
3. 大小写查询：`SELECT word FROM dict_word WHERE word='BRAIN' COLLATE NOCASE LIMIT 1;`

## 假设与决策

1. **假设**：用户机器上 Python 可用（用于一次性预处理）。若不可用，可用 Rust 写一个 `tools/build_dict` 二进制完成同样工作（备选方案，不在本期实施）
2. **决策**：词典库失败时**不**阻断应用启动，只让 `dict_lookup_*` 命令返回错误，UI 显示"词典未就绪"
3. **决策**：双击只查英文单词（正则 `^[A-Za-z][A-Za-z'-]*$`），不查英文短语
4. **决策**：选中中文时查中→英，关键词长度限制 30 字符（避免用户选一整段触发全表扫描）
5. **决策**：中文查询用 LIKE '%keyword%' 全表扫描，7 万行约 20-50ms，可接受。后续不够再升级 FTS5
6. **决策**：浮窗位置跟随选中点（`event.clientX/clientY`），不使用 element-plus 的 el-popover（它基于触发元素，不适合双击/划选场景）
7. **决策**：词典库只读，永不写入。升级词典时替换 `resources/dictionary.db` 文件即可
8. **决策**：`book.csv` + `relation_book_word.csv`（116MB）**不**纳入本期，用户明确未要求词书功能
9. **决策**：含空格的英文短语（如 "pick up"）本期忽略，不弹窗；后续可扩展支持

## 验证步骤

完成实施后按以下顺序验证：

1. **词典库生成**：
   ```bash
   python scripts/build_dictionary_db.py
   ```
   输出 `src-tauri/resources/dictionary.db`，文件存在且大小 5-15MB

2. **Rust 编译**：
   ```bash
   cd src-tauri && cargo check
   ```
   无错误无警告

3. **前端构建**：
   ```bash
   npm run build
   ```
   无 TS 类型错误

4. **运行时验证**（`npm run tauri dev`）：

   **英→中**：
   - 双击英文单词（如 "brain"），浮窗出现，显示音标和中文释义
   - 划选英文单词（按住拖动）也能弹窗
   - 词典无此词时显示"词典中无此词"
   - 词汇本下拉能列出所有 vocab_book
   - 点击"加入词汇本"成功，词汇本详情页能看到该词
   - 重复加入同一词，提示"单词已存在"

   **中→英**：
   - 划选中文词（如 "脑"），浮窗出现，显示匹配的英文单词列表
   - 列表按词频排序，每条带音标和释义
   - 每条右侧有独立"加入"按钮
   - 点击某条的"加入"成功后，该条按钮变为"已加入"，其他条目仍可加入
   - 选中的中文无匹配时显示"未找到对应英文单词"

   **通用**：
   - 双击中文不弹窗（中文靠划选触发）
   - 划选英文短语（如 "pick up"）不弹窗
   - 点击浮窗外区域，浮窗关闭
   - 选中文本长度超 30 字符时不弹窗（避免误操作）

5. **打包验证**（可选）：
   ```bash
   npm run tauri build
   ```
   检查安装包大小合理（增加约 10MB），安装后查词功能正常

## 文件清单

### 新建
- `scripts/build_dictionary_db.py` — 数据预处理脚本
- `src-tauri/resources/dictionary.db` — 生成的词典库（脚本产物）
- `src-tauri/src/dictionary.rs` — 词典连接 + 命令
- `src-tauri/src/models/dict_word.rs` — 响应结构体
- `src/stores/dictionaryStore.ts` — Pinia 词典状态
- `src/components/novel/DictLookupPopover.vue` — 浮窗组件

### 修改
- `src-tauri/tauri.conf.json` — 添加 `bundle.resources`
- `src-tauri/src/lib.rs` — 注册 dict 模块、managed state、命令
- `src-tauri/src/models/mod.rs` — 导出 DictWord
- `src/components/novel/NovelEditor.vue` — 挂双击事件、集成浮窗

## 工作量评估

| 模块 | 内容 |
|------|------|
| 数据预处理 | Python 脚本一次跑完 |
| 后端 | 1 个模块 + 1 个命令 + 1 个结构体 |
| 前端 | 1 个 store + 1 个组件 + 编辑器集成 |
| 验证 | cargo check + npm run build + 手动双击测试 |
