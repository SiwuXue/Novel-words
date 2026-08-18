# 精读版 Step 开关（Step 1/2/3 可配置导出）— 任务清单 tasks.md

执行顺序建议：**后端 → 类型/Store → 设置页 UI → 编辑页 UI → 预览渲染 → 验证**。

---

## ✅ Task 1：Rust 后端基础（DB seed + IntensiveSteps 结构 + 解析）

- 优先级：P0
- 预估时间：15 min
- 产出文件：
  - `src-tauri/src/db.rs` — seed 区加一行 `INSERT OR IGNORE INTO app_settings (key, value) VALUES ('pdf_intensive_steps', '[1,2,3]');`
  - `src-tauri/src/pdf/intensive.rs` — 新增 `IntensiveSteps` 结构体（Clone/Copy/Debug/Default）、`any()`、`normalize()`、`parse_steps_from_db(Option<&str>)`
- 验收点：`cargo check` 通过；结构体字段 `step1/step2/step3: bool`；`parse_steps_from_db(None)` 返回 `{ step1:true, step2:true, step3:true }`；`parse_steps_from_db(Some("[1,3]"))` 返回 step1+3。
- 注意：依赖 crate serde 已引入则用 serde_json，否则手写最小 JSON 数组解析（按字符 `[`, `]`, `,` 分割数字即可，避免加新依赖）。

---

## ✅ Task 2：Rust 渲染调用链改造（render / generate_pdf / export_pdf）

- 优先级：P0
- 预估时间：25 min
- 产出文件：
  - `src-tauri/src/pdf/intensive.rs` — `pub fn render(ctx, chapters, vocabs, steps: IntensiveSteps)`；Step1/2/3 三块分别 `if steps.stepN { ... }`；章节结束 marker 始终保留
  - `src-tauri/src/pdf/mod.rs` — `pub fn generate_pdf(novel, template, vocabs, chapters, steps: IntensiveSteps, output_path)` 把 steps 传入 `intensive::render`
  - `src-tauri/src/commands/pdf_export.rs` — 命令新增 `steps: Option<Vec<i64>>` 参数；优先级：前端传值 → DB 读默认 → normalize；最后传给 `pdf::generate_pdf`
- 验收点：`cargo check` 0 warn 0 err；不传 steps 时 DB 兜底逻辑正确；前端传 steps 覆盖逻辑正确；全不选时 normalize 强制 step1=true。
- 注意：`db.query_row` 读 `app_settings` 的时机必须在 `state.db.lock()` 持锁期间（与 novel/vocabs 查询同一块），避免再次加锁。

---

## ✅ Task 3：前端共享类型 + Pinia Settings Store 扩展

- 优先级：P0
- 预估时间：20 min
- 产出文件：
  - 新建 `src/types/pdfSteps.ts` — `StepNum` 类型、`ALL_STEPS`、`STEP_LABELS`、`normalizeSteps`、`serializeSteps`
  - `src/stores/settingsStore.ts` — 新增 `pdfIntensiveSteps: ref<StepNum[]>([1,2,3])`、`load()` 里增加 `pdf_intensive_steps` case、新增 `setPdfIntensiveSteps(steps: StepNum[])` 方法；export 暴露出来
- 验收点：`normalizeSteps(null)` / `normalizeSteps([])` → 返回 `[1,2,3]`；`normalizeSteps([1,3,3,9])` → `[1,3]`；setPdfIntensiveSteps 正确写入 Pinia 和 DB。
- 注意：`normalizeSteps` 对重复值去重、对非 1/2/3 过滤、按升序输出。

---

## ✅ Task 4：SettingsPage.vue — 设置页「精读版导出步骤」UI

- 优先级：P0
- 预估时间：15 min
- 产出文件：`src/views/SettingsPage.vue`
- UI 实现：
  - 在「默认词汇本」表单项下方新增 `el-form-item label="精读版导出步骤"`
  - 内容为 `<el-checkbox-group v-model="localSteps" @change="onStepsChange">` + 三个 `el-checkbox` label 用 `STEP_LABELS[1/2/3]`
  - 用 `ref localSteps` 做本地缓冲（onMounted 时从 store 赋值），避免 change 中 rollback 反向影响 store
  - `onStepsChange`：若空数组 → ElMessage.warning 并 `localSteps.value = settingsStore.pdfIntensiveSteps` rollback；否则调 store.setPdfIntensiveSteps
- 验收点：默认全选；勾选改了立即生效；空选拒绝且不写 DB；页面刷新保留
- 注意：使用 `settingsStore.loaded` 或 `onMounted` 保证初始化顺序。

---

## ✅ Task 5：NovelEditorPage.vue — 导出步骤选项与传参

- 优先级：P0
- 预估时间：25 min
- 产出文件：`src/views/NovelEditorPage.vue`
- UI / 数据：
  - 引入 `normalizeSteps` 和 `STEP_LABELS`
  - 新增 `const pdfSteps = ref<StepNum[]>([1,2,3])`；在 `settingsStore.loaded === true` 时或 `watch` 设置 store 变化，**一次性同步初始值**到 pdfSteps（拷贝）
  - 在 `导出 PDF` 按钮左侧新增 `el-dropdown`（link 样式）：`⚙ 导出步骤：已选 ${pdfSteps.value.length} 项`；下拉面板内部放 3 个 el-checkbox-group（独立绑定 pdfSteps）
  - `previewHtml` computed 里调用 `buildPreviewHtml` 追加 `steps: normalizeSteps(pdfSteps.value)` 兜底
  - `handleExportPdf` 里 invoke 追加 `steps: normalizeSteps(pdfSteps.value)`
- 验收点：pdfSteps 变化 → 预览立即刷新；导出的 PDF 与勾选一致；设置页改动只影响新的编辑页会话（不回写已经打开的编辑页）。
- 注意：不要让 pdfSteps 反向写回 settingsStore（是会话级覆盖，不要持久化）。

---

## ✅ Task 6：pdfPreview.ts — 按 steps 分支渲染

- 优先级：P0
- 预估时间：15 min
- 产出文件：`src/utils/pdfPreview.ts`
- 修改点：
  - 导入 `type StepNum` 和 `normalizeSteps`
  - `BuildPreviewInput` 增加 `steps?: StepNum[]`（可选）
  - `buildHtml(input)` 中 `const steps = normalizeSteps(input.steps)`，传给 `buildIntensive`
  - `buildIntensive(chapters, words, _novelTitle, steps = [1,2,3] as StepNum[])` 新增第 4 参数
  - 三块 Step 对应代码分别 `if steps.includes(1/2/3)` 包裹；章节结束 marker 保持始终
- 验收点：不传 steps → 行为与原版一致（向后兼容）；只传 `[3]` → 仅输出 Step 3。
- 注意：不要把步骤描述 / 标题 / 段落渲染混进 if 语句外，否则「只选 Step3」时仍会打印出 Step1 的标题。

---

## ✅ Task 7：端到端验证（Build + Checklist 抽样）

- 优先级：P0
- 预估时间：20 min
- 命令：
  1. `npm run build`
  2. `cargo check --manifest-path src-tauri/Cargo.toml`
- 人工走查 checklist 中至少：#1 / #4 / #6~9 / #12 / #13 / #14 / #15 / #18 / #19 / #20（覆盖功能 + 编译 + 兼容性）。
- 验收点：checklist Blocking 项（❗）全部勾选通过；Build 0 error；Cargo 0 warning。

---

## 📋 总览

| Task | 描述 | 预估耗时 |
|---|---|---|
| Task 1 | Rust 后端基础（DB seed + IntensiveSteps 结构 + 解析） | 15 min |
| Task 2 | Rust 渲染调用链改造（render / generate_pdf / export_pdf） | 25 min |
| Task 3 | 前端共享类型 + Pinia Settings Store 扩展 | 20 min |
| Task 4 | SettingsPage.vue — 设置页步骤勾选 UI | 15 min |
| Task 5 | NovelEditorPage.vue — 导出步骤选项与传参 | 25 min |
| Task 6 | pdfPreview.ts — 按 steps 分支渲染 | 15 min |
| Task 7 | 端到端验证（Build + Checklist 抽样） | 20 min |
| **合计** | | **~2h** |
