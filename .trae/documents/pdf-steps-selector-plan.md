# 精读版 Step 开关（Step 1/2/3 可配置导出）实现计划

> 创建：2026-08-18  
> 关联：精读版三阶段结构重构、Step 3 单词列表、AppSetting 默认偏好

## 1. 目标

- 用户可以在**设置页**配置精读版的默认导出步骤（Step 1 / Step 2 / Step 3），默认全开。
- 用户可以在**小说编辑页**覆盖默认偏好，勾选本次导出 / 预览包含的 Step。
- 章标题（Chapter N / 标题 / 第 N 章 / 本章词汇 N 词）与章节结束标记「—— 第 N 章 完 ——」**始终显示**，不属于 Step 开关范围。
- 三步全不选时，前端给出「至少选一个」提示，后端兜底按 Step 1 渲染。

## 2. 数据模型（DB / 存储）

- **表结构不变**：继续用 `app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)`。
- **新增 key**：`pdf_intensive_steps`，value 格式为 JSON 数组字符串，例 `"[1,2,3]"`。有效元素为 `1`、`2`、`3`；忽略其他值；空数组等同于未设置。
- **DDL Seed**：在 `src-tauri/src/db.rs` 末尾的 seed 区域新增：
  ```rust
  INSERT OR IGNORE INTO app_settings (key, value) VALUES ('pdf_intensive_steps', '[1,2,3]');
  ```
- **读取路径**：
  - 前端：Pinia `settingsStore` 的 `load()` 里读，反序列化到 `pdfIntensiveSteps: StepNum[]`。
  - Rust 后端：`export_pdf` 命令里在释放 `DbState` 锁之前读取 `pdf_intensive_steps` 一次，作为默认值。

## 3. 共享类型与工具（前端）

新建文件 `src/types/pdfSteps.ts`：

```ts
/**精读版步骤编号：1=Step1 语境背词, 2=Step2 回忆词义, 3=Step3 单词列表 */
export type StepNum = 1 | 2 | 3

export const ALL_STEPS: StepNum[] = [1, 2, 3]

export const STEP_LABELS: Record<StepNum, string> = {
  1: 'Step 1：在语境中背单词',
  2: 'Step 2：看单词回忆词义',
  3: 'Step 3：单词列表',
}

/**把任意输入规范化为合法 StepNum 数组，空/非法时返回 [1,2,3] */
export function normalizeSteps(value: unknown): StepNum[] {
  if (!Array.isArray(value)) return [...ALL_STEPS]
  const set = new Set<StepNum>()
  for (const v of value) {
    const n = Number(v)
    if (n === 1 || n === 2 || n === 3) set.add(n)
  }
  const arr = [...set].sort((a, b) => a - b)
  return arr.length ? arr : [...ALL_STEPS]
}

/**把合法步骤数组序列化为 app_setting 的 value 字符串 */
export function serializeSteps(steps: StepNum[]): string {
  return JSON.stringify([...new Set(steps)].sort((a, b) => a - b))
}
```

## 4. Pinia 设置状态扩展

文件：`src/stores/settingsStore.ts`

- 新增 `pdfIntensiveSteps: ref<StepNum[]>([1,2,3])`
- 在 `load()` 的 for 循环里增加 case：
  ```ts
  case 'pdf_intensive_steps': {
    try { pdfIntensiveSteps.value = normalizeSteps(JSON.parse(s.value)) }
    catch { /* 非法 JSON → 保持默认 */ }
    break
  }
  ```
- 新增 `setPdfIntensiveSteps(steps: StepNum[])` 方法：normalize → 写本地 ref → `invoke('set_setting', { key: 'pdf_intensive_steps', value: serializeSteps(steps) })`
- 导出里新增相应字段和方法。

## 5. SettingsPage.vue — 设置页 UI

- 「通用」标签页里，**默认词汇本**的表单项下方新增一个 `el-form-item label="精读版导出步骤"`，放三个 `el-checkbox`：
  ```
  ☐ Step 1：在语境中背单词   ☐ Step 2：看单词回忆词义   ☐ Step 3：单词列表
  ```
- 使用 `settingsStore.pdfIntensiveSteps` 做 `v-model`（Checkbox Group 模式）。
- change 事件里调 `settingsStore.setPdfIntensiveSteps(...)`。
- 校验：change 时如果数组长度为 0，用 `ElMessage.warning('至少勾选一个步骤')`，并把值还原为上次的合法值（不要清空）。

## 6. NovelEditorPage.vue — 小说编辑页 UI 与数据流

### 6.1 顶部工具栏新增「导出选项」
- 在 `<el-button 导出PDF>` 按钮左侧插入一个小按钮（`el-button` link 类型）或 `el-dropdown`，文字：「⚙ 导出步骤：已选 3 项」（数量动态显示）。
- 下拉面板内放三个 Checkbox，与 SettingsPage 同构，初始值从 `settingsStore.pdfIntensiveSteps` 拷贝到一个本地 `ref pdfSteps`（只影响本次会话，不回写 DB）。
- `previewHtml` 计算属性会监听 `pdfSteps` 并自动重新渲染预览。

### 6.2 预览传参
- 修改 `buildPreviewHtml({ chapters, words, novelTitle, steps })` 调用时增加 `steps: pdfSteps.value` 参数。
- `NovelEditorPage` 里：若 `pdfSteps.length === 0`，用 `normalizeSteps` 兜底为 `[1,2,3]`。

### 6.3 导出传参
- `handleExportPdf()` 里 invoke `export_pdf` 时追加参数：
  ```ts
  steps: pdfSteps.value,   // StepNum[]，前端保证至少 1 项
  ```

## 7. 前端预览渲染层修改

文件：`src/utils/pdfPreview.ts`

- `BuildPreviewInput` 增加可选字段 `steps?: StepNum[]`。
- `buildHtml(input)` 中把 `steps` 传给 `buildIntensive`。
- `buildIntensive(chapters, words, _novelTitle, steps = [1,2,3])` 新增第四参数：
  ```ts
  function buildIntensive(chapters, words, _novelTitle, steps) {
    const includeStep1 = steps.includes(1)
    const includeStep2 = steps.includes(2)
    const includeStep3 = steps.includes(3)
    // ...
    // Step 1 块整体 wrap：if (includeStep1) { parts.push(step1 标题 + 描述 + 段落 + step1 end) }
    // Step 2 块整体 wrap：if (includeStep2) { parts.push(step2 标题 + 描述 + 段落) }
    // Step 3 块整体 wrap：if (includeStep3) { parts.push(buildStep3Block(chWords)) }
    // 章节结束标记始终保留
  }
  ```

## 8. Rust 后端渲染层修改

### 8.1 新增结构体 IntensiveSteps
- 在 `src-tauri/src/pdf/intensive.rs`（或 `src-tauri/src/models`）加：
  ```rust
  #[derive(Clone, Copy, Debug, Default)]
  pub struct IntensiveSteps {
      pub step1: bool,
      pub step2: bool,
      pub step3: bool,
  }
  impl IntensiveSteps {
      pub fn any(self) -> bool { self.step1 || self.step2 || self.step3 }
      pub fn normalize(mut self) -> Self {
          if !self.any() { self.step1 = true; }
          self
      }
  }
  ```
- 增加解析函数 `parse_steps_from_db(value: Option<&str>) -> IntensiveSteps`：
  - DB 里没设置/解析失败/数组空 → 返回 `{ step1: true, step2: true, step3: true }`
  - 否则按数组元素 `1/2/3` 对应 flag。

### 8.2 `intensive::render` 签名改造
```rust
pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord], steps: IntensiveSteps)
```
- 章节循环里，Step 1 / Step 2 / Step 3 三个代码块各自用 `if steps.step1 { ... }` 等包起来。
- 章节结束标记 `draw_chapter_end_marker(...)` 始终调用。

### 8.3 `pdf::generate_pdf` 签名改造
```rust
pub fn generate_pdf(
    novel: &Novel,
    template: &PdfTemplate,
    vocabs: &[VocabWord],
    chapters: &[Chapter],
    steps: IntensiveSteps,
    output_path: &str,
) -> Result<(), String>
```
- 内部把 `steps` 传入 `intensive::render(...)`。

### 8.4 `commands/pdf_export.rs` 命令层改造
- `export_pdf` 新增参数 `steps: Option<Vec<i64>>`（前端传）。
- **优先级**：前端传 `steps` → 直接用；否则从 `app_settings` 读 `pdf_intensive_steps` key → 解析；兜底全开。
- 在持锁期内（`db` 变量活着时）一次性执行：
  ```rust
  let steps = {
      let parsed = steps.and_then(|arr| Some(IntensiveSteps {
          step1: arr.contains(&1),
          step2: arr.contains(&2),
          step3: arr.contains(&3),
      })).unwrap_or_else(|| {
          let db_val: Result<String, _> = db.query_row(
              "SELECT value FROM app_settings WHERE key='pdf_intensive_steps'",
              [], |row| row.get(0),
          );
          parse_steps_from_db(db_val.ok().as_deref())
      });
      parsed.normalize()
  };
  ```
- 调用 `pdf::generate_pdf(..., steps, &output_path)`。

## 9. 前端校验 & 错误处理
- SettingsPage change → 全不选 → `ElMessage.warning('至少勾选一个步骤')` 并 rollback。
- NovelEditorPage pdfSteps change → 同上策略，避免空白 PDF。
- Rust 端 `IntensiveSteps::normalize()` 兜底 `step1 = true`，避免 panic。

## 10. 验证清单（Checklist）
- [ ] 设置页「精读版导出步骤」3 个 Checkbox 默认全选；取消 Step1 重开设置后依然生效
- [ ] 改了默认步骤后：重启 App，设置页仍保持（验证 DB 写入）
- [ ] 只勾选 Step 1：预览 / 导出 PDF — 只有 Step1 块，章节结束保留
- [ ] 只勾选 Step 2：预览 / 导出 PDF — 只有 Step2 块
- [ ] 只勾选 Step 3：预览 / 导出 PDF — 只有 Step3 块（无正文段落，直接到表格）
- [ ] 小说编辑页临时覆盖（如只选 Step1+3）：预览和导出一致，**不影响**设置页里的默认值
- [ ] 三步全不选（强行尝试）：UI 拒绝变更并提示
- [ ] `npm run build` 通过
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` 0 err 0 warn

## 11. 实施任务拆分（tasks.md）
1. **后端（Rust）** — DB seed、IntensiveSteps 结构/解析、render/generate_pdf/export_pdf 签名与调用链改造
2. **前端类型/Store** — StepNum 类型、normalize/serialize 工具、settingsStore 新增字段和方法
3. **前端设置页** — SettingsPage.vue 步骤勾选 UI
4. **前端编辑页** — NovelEditorPage.vue 导出步骤选项与传参
5. **预览/后端渲染** — pdfPreview.ts 和 pdf/intensive.rs 按开关渲染
6. **验证** — 前端类型检查 + Rust check，跑 checklist

## 12. 风险与兼容性
- 老用户升级：首次进入读不到 `pdf_intensive_steps`，两端兜底返回 `[1,2,3]`，等价于原行为，无感升级。
- 前端 `buildHtml` 的 `steps` 参数做可选 + 默认值，外部调用方（测试/其他页面）不传参则不破坏。
- `export_pdf` 命令的 `steps` 参数用 `Option`，不传则保持老流程，不破坏脚本化调用。
