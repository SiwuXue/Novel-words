# 精读版 Step 开关（Step 1/2/3 可配置导出）— 规格说明书 spec.md

## 1. 背景 & 目标

精读版已固定为三阶段学习结构：Step 1（语境背词）、Step 2（回忆词义）、Step 3（单词列表）。部分用户希望按场景组合输出 —— 比如只导出 Step 2 做回忆训练、只导出 Step 3 做单词复习。本规格说明如何给精读版加上 Step 可配置开关。

## 2. 角色与场景

| 场景 | 触发者 | 操作 | 预期结果 |
|---|---|---|---|
| 默认偏好设置 | 用户（SettingsPage） | 勾选 Step 1/2/3 后保存 | 每次打开设置值与上次一致；新会话 / 导出命令默认使用 |
| 单次覆盖 | 用户（NovelEditorPage） | 点开「⚙ 导出步骤」下拉调整勾选 | 预览和本次导出按勾选输出，**不影响**设置页默认 |
| 老用户升级 | 系统 | 首次进入 | 两端兜底，等价于全部 Step 启用（无感升级） |

## 3. 功能需求

### FR-1 AppSetting 持久化
- Key：`pdf_intensive_steps`，类型 TEXT。
- Value 规范：JSON 编码的数字数组。合法元素 `1`、`2`、`3`。其余元素 / 非法 JSON / 空数组 → 读取时按「默认 [1,2,3]」处理。
- 数据库种子：`INSERT OR IGNORE … VALUES ('pdf_intensive_steps', '[1,2,3]')`，已存在的用户不会被覆盖。

### FR-2 设置页 UI（SettingsPage.vue）
- 「通用」标签页新增表单项 `精读版导出步骤`，三个 `el-checkbox` 绑定到 Pinia `settingsStore.pdfIntensiveSteps`（Checkbox Group 模式）。
- Checkbox 文案：
  - `Step 1：在语境中背单词`
  - `Step 2：看单词回忆词义`
  - `Step 3：单词列表`
- 变更时若选空数组 → `ElMessage.warning('至少勾选一个步骤')` 并 rollback，不写 DB。
- 合法变更 → `settingsStore.setPdfIntensiveSteps(...)` 立即写 Pinia + DB。

### FR-3 小说编辑页（NovelEditorPage.vue）单次覆盖
- 顶栏「导出 PDF」按钮左侧新增「⚙ 导出步骤：N 项」按钮（el-dropdown 或 link-style）。
- 下拉面板：与设置页同构的三个 Checkbox，绑定到本地 `ref pdfSteps`。
- **初始化**：页面挂载 / `settingsStore.loaded` 变为真时，把 `settingsStore.pdfIntensiveSteps` 克隆到 `pdfSteps`。
- **预览联动**：`previewHtml` 计算属性依赖 `pdfSteps`，勾选变更后预览实时重渲染。
- **导出传参**：`invoke('export_pdf', { ..., steps: pdfSteps.value })`，前端再次兜底至少 1 项。

### FR-4 前端预览渲染（pdfPreview.ts）
- `BuildPreviewInput` 加字段 `steps?: StepNum[]`。
- `buildHtml(input)` 不传 `steps` → `steps = [1,2,3]`（向后兼容）。
- `buildIntensive(chapters, words, _novelTitle, steps)` 按 steps.includes 开关 Step 1/2/3 块；章标题 & 章节结束标记保持始终显示。

### FR-5 Rust 后端渲染
- `IntensiveSteps { step1: bool, step2: bool, step3: bool }` 结构体 + `normalize()`（全 false 时 `step1 = true`）。
- `parse_steps_from_db(Option<&str>)`：DB 字符串 → flags；缺省 / 空 / 非法 → 全开。
- `intensive::render(ctx, chapters, vocabs, steps)`：三步块分别用 `if steps.stepN` 包起来；章结束标记始终调用。
- `pdf::generate_pdf(... steps, output_path)` 新增参数并下传给 `render`。
- `export_pdf` 命令新增 `steps: Option<Vec<i64>>`：优先用前端传，空则读 DB 默认值，再调 normalize。

## 4. 非功能需求

- **兼容**：`buildHtml`、`export_pdf` 的新增参数为可选 / `Option`，外部调用不传参保持旧行为。
- **鲁棒**：任何非法 steps 输入（全空、非数组、非数字）不抛异常，按默认或 normalize 后继续。
- **一致**：前端预览 HTML 与后端 PDF 产出必须严格使用同一套开关逻辑，勾选 → 预览 → 导出三者一致。
- **可维护**：共享 `StepNum` 类型和 normalize/serialize 工具，避免各组件重复硬编码。

## 5. 输出产物

- DB seed 行 `pdf_intensive_steps = [1,2,3]`
- 前端：`src/types/pdfSteps.ts`、`settingsStore` 扩展、`SettingsPage.vue` + `NovelEditorPage.vue` UI
- 预览：`pdfPreview.ts` 按 steps 分支渲染
- 后端：`IntensiveSteps` 结构 + `parse_steps_from_db`、`render/generate_pdf/export_pdf` 签名与逻辑改造

## 6. 不在范围内（明确排除）

- 不新增 DB 表结构（沿用 `app_settings`）。
- 不支持为每个 Chapter 单独设置不同的步骤开关（全局粒度 / 单次导出粒度即可）。
- 不支持步骤之间自定义顺序（固定 1→2→3，勾选哪些就按序号渲染）。
- 不修改 Step 3 的分页算法（按顺序双列，保持现状；如需四象限交错另开需求）。
