# UI 验证

```powershell
npm test
npm run build
npx playwright install chromium
npm run test:ui
```

单元测试覆盖导航归属、阅读入口、侧面板互斥、整本进度计算、加载失败与原生拖入事件。浏览器测试使用仅限测试的 Tauri bridge，覆盖阅读/编辑切换、位置恢复、导出失败重试、键盘操作及各页面在浅色/深色、中英文和三个窗口尺寸下的布局。长列表样本包含 150 本小说、150 个词汇本、1,000 个单词；长章节样本超过 200KB。

## Tauri 实测

先启动 Vite，再使用独立测试应用标识启动 Tauri，避免测试数据进入日常数据库：

```powershell
npm run dev
```

在另一终端运行：

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9229'
npm run tauri dev -- --config '{"identifier":"com.tauri-app.novel-words-ui-test","build":{"beforeDevCommand":""}}'
```

连接实际 WebView2，验证导入识别、查词收词、编辑保存、复习持久化及 Rust PDF 生成：

```powershell
node tests/native-smoke.mjs
```

脚本检查测试应用标识，创建临时小说与词汇本，并在 `finally` 中删除这些临时数据。默认使用原生拖入事件，直接调用现有 PDF 命令；生成的测试文件位于 `test-results/`。

解锁桌面后，可另外检查原生文件选择和 PDF 保存窗口：

```powershell
$env:NATIVE_PICKERS='1'
node tests/native-smoke.mjs
```

该模式会输出要选择的小说路径及 PDF 保存路径，每个文件窗口最多等待三分钟，需要在窗口中选择对应路径。不要与会清空 `test-results/` 的其他 Playwright 运行同时执行。
