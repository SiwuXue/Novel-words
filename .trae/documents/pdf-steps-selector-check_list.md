# 精读版 Step 开关（Step 1/2/3 可配置导出）— 验收清单 check_list.md

> 勾选 [x] 代表通过；勾 [ ] 代表未通过 / 未测；带 ❗ 的属于 Blocking 问题，必须在本任务内修复。

---

## ✅ 功能验收

| # | 检查项 | 状态 | 备注 |
|---|---|---|---|
| 1 | 全新安装 / 首次启动 → 设置页「精读版导出步骤」三个 Checkbox 默认全选 | [ ] | 默认 seed `[1,2,3]` |
| 2 | 取消勾选 Step 1 → 保存后关闭 App → 重开 → 设置页仍取消 Step 1（DB 持久化验证） | [ ] | |
| 3 | 设置页若取消到全部为空 → `ElMessage.warning('至少勾选一个步骤')` 且值还原，不写 DB | [ ] | rollback 正确性 |
| 4 | 小说编辑页：顶栏「⚙ 导出步骤：N 项」下拉可勾选；下拉勾选后立即触发预览 HTML 重新生成 | [ ] | computed 依赖正确 |
| 5 | 小说编辑页临时改成只选 Step 1 → 设置页默认值保持不变（互不影响） | [ ] | 本地 ref 隔离 |
| 6 | 预览：只选 Step 1 → 仅显示 Step1 区块，不显示 Step2 / Step3；章节结束标记存在 | [ ] | |
| 7 | 预览：只选 Step 2 → 仅显示 Step2 区块 | [ ] | |
| 8 | 预览：只选 Step 3 → 仅显示 Step3 双列表格（无正文段落）；表头顶灰蓝、边框、熟练度着色 | [ ] | |
| 9 | 预览：只选 Step 1+3 → 中间 Step 2 空缺不显示，顺序正确 | [ ] | 顺序校验 1→3 |
| 10 | 导出 PDF（对应 6-9 的勾选组合）：打印 PDF 内容与预览严格一致 | [ ] | 多组合抽测 |
| 11 | 非法输入防御：前端传 steps=[] / 非数组 → 后端按 normalize() 兜底输出 Step 1，不 panic | [ ] | |

---

## 🔧 工程 / 代码质量验收

| # | 检查项 | 状态 | 备注 |
|---|---|---|---|
| 12 | `src/types/pdfSteps.ts` 存在；`StepNum / ALL_STEPS / STEP_LABELS / normalizeSteps / serializeSteps` 导出并通过类型检查 | [ ] | |
| 13 | `settingsStore.pdfIntensiveSteps` 类型正确、load() 解析 case、setPdfIntensiveSteps 方法存在 | [ ] | |
| 14 | `db.rs` seed 有 `INSERT OR IGNORE INTO app_settings (key, value) VALUES ('pdf_intensive_steps', '[1,2,3]')` | [ ] | |
| 15 | Rust `IntensiveSteps` 有 Clone/Copy/Debug、`any()`、`normalize()`（全 false → step1=true） | [ ] | |
| 16 | `intensive::render / pdf::generate_pdf / export_pdf` 调用链签名统一改造，无遗漏参数 | [ ] | 无 missed calls |
| 17 | 所有新增 / 修改的步骤判定逻辑集中化，不出现硬编码 `1/2/3` 魔数散落 3 处以上 | [ ] | 维护性 |
| 18 | `npm run build` → 0 error；如有 TS / ESLint 报错必须修复 | [ ] | |
| 19 | `cargo check --manifest-path src-tauri/Cargo.toml` → 0 error / 0 warning；有 warning 必须说明并修复 | [ ] | Rust 死代码警告零容忍 |
| 20 | 老用户兼容性：DB 里没有 `pdf_intensive_steps` key → 两端兜底为全开 [1,2,3]，等价于改造前行为 | [ ] | 向上兼容 |

---

## 🚨 阻塞性问题（必须修复）

- ❗ 任何一步导致预览与 PDF 内容不一致（steps 开关不同步）
- ❗ Rust 编译报错或 warning
- ❗ 设置页勾选后不持久化（刷新即丢失）
- ❗ 三步全不选时渲染出空白章节导致异常分页 / NPE
