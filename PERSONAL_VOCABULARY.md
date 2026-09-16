# 预设词表与个人总词汇库

预设目录包含 13 类考试合集和 27 套教材分册，安装后离线使用。首次使用一套词表时加载资源并缓存；目录中的词数是该套资源的实际去重词数。资源来源、归并规则和重建方式见 [PRESETS.md](src-tauri/resources/PRESETS.md)。

“整套导入”建立个人词汇本；“小说精选”继续使用现有的小说匹配流程。导入结果分别显示新增个人单词、继承已有状态和同本重复跳过的数量。

## 共享学习状态

个人学习记录保存在现有 SQLite 文件的 `user_vocab` 表。同一个词在各词汇本中共享熟练度和 SRS 计划，释义、例句、标签及小说来源仍属于各词汇本。词汇本页面的“全部词汇”标签页支持分页、搜索、筛选、单个和批量标记，并可查看所在词汇本。

单词键忽略大小写，清除首尾空白、合并连续空白并统一弯引号。其他标点和词形保留，例如 `run` 与 `running` 是两条个人记录。已有个人状态优先于导入状态；只有新个人词才采用有效的初始状态。改名单词时，已存在的目标词继承它的学习状态，新目标词采用用户明确选择的初始熟练度。

删除最后一个词汇本来源后，学习记录和复习历史仍保留，总词数仍计入该词，但待复习队列暂停收录。再次导入后按原计划恢复。全部复习按个人记录去重，单本复习保留该本的例句和来源。

## 迁移与恢复

首次迁移前自动生成完整 `backups/pre-user-vocab-*.db` 备份。个人词汇迁移在事务内完成，并保留现有复习事件；同本重复行的非空内容会补入保留的行。冲突优先采用可靠时间较新的学习结果，同秒复习以日志顺序判断；无可靠时间时优先最高熟练度及有效计划。

恢复备份时先在临时副本中迁移并检查完整性和外键，成功后才恢复当前连接，前端随后刷新。所选备份文件不被改写，校验失败保持当前数据。

## 验证

```powershell
npm test
npm run test:ui
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
python -m unittest discover -s scripts -p test_prepare_presets.py
```

原生端到端脚本仅允许标识符为 `com.tauri-app.novel-words-vocab-test` 的独立应用，不连接日常使用的数据目录。先启动 Vite，再启动专用 Tauri 实例：

```powershell
npm run dev
# 在另一终端运行：
'{"identifier":"com.tauri-app.novel-words-vocab-test","build":{"beforeDevCommand":""}}' | Set-Content vocab-smoke.local
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9229'
npx tauri dev --config vocab-smoke.local
```

```powershell
python tests/prepare-native-vocabulary.py
node tests/native-user-vocabulary.mjs 1
# 关闭并重新启动上述 Tauri 实例后：
node tests/native-user-vocabulary.mjs 2
```

脚本验证 40 个目录、整套/手动/CSV/阅读/小说精选入口、跨本共享、删除暂停、重导入、备份恢复、重启、阅读位置及 Rust PDF 生成。验证产物保存在已忽略的 `test-data/native-vocabulary` 目录，避免与 Playwright 自动清理的输出目录冲突。
