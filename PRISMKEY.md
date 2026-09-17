# 词阅卡密接入

词阅使用独立产品。PrismKey 原来的 XHS_Download 卡密不能激活词阅。

| 配置 | 默认值 |
| --- | --- |
| 服务 | `https://license.wuyiuou.top` |
| 产品 slug | `novel-words` |
| 产品 ID / JWT audience | `NovelWords` |
| JWT issuer | `PrismKey_NovelWords` |
| 卡密前缀 | `CY` |
| 签名 | Ed25519 / EdDSA |
| 内置公钥 kid | `4906a9a5e11eef9e` |

## 使用

首次启动输入词阅卡密。设置中的“词阅授权”页展示脱敏卡密、套餐、到期时间和校验状态，可以重新校验或更换卡密。换卡失败保留已有有效授权。

支持 7 天、30 天、365 天和永久套餐。首次激活绑定设备，期限以服务端为准；重复激活、解绑后重新绑定都不重新计算套餐起点。

启动时由 Rust 发起校验；前端也在初始化、恢复前台和定时检查时刷新状态。核心 Tauri 命令统一检查授权。未激活、停用或到期时展示激活页，个人小说和学习记录不删除，仍能备份个人数据库、调整主题和语言。已打开的编辑器锁定时保留组件与文本；仅此前读取的正文允许短暂完成待保存内容，最长 60 秒。

网络无法连接或服务端 5xx 时，已签名凭证可以离线使用，最迟到上次签发后 24 小时，并且不超过套餐到期时间。离线、重启和重复本地查询都不会延长期限。停用、解绑、设备不匹配、限流、无效签名及错误响应不触发离线放行。系统时间回退要求联网校验。

JWT 在 Rust 验证固定公钥、kid、issuer、audience、设备摘要、套餐和时间；前端只收到状态与脱敏信息。激活及校验只发送 `card_key`、`device_id`、`client_version`，符合 supplied PrismKey 的请求 schema。

## 服务端注册独立产品

在可信的 PrismKey 部署连接上先列出产品，确认没有同名或身份冲突的记录，再执行现有 CLI：

```sh
python -m app.cli list-products
python -m app.cli add-product --slug novel-words --name 词阅 --product-id NovelWords --issuer PrismKey_NovelWords --prefix CY
```

北京 Docker 部署使用：

```sh
docker compose --project-directory /opt/prismkey -f /opt/prismkey/compose.yaml -f /opt/prismkey/compose.nginx.yaml exec -T api python -m app.cli list-products
docker compose --project-directory /opt/prismkey -f /opt/prismkey/compose.yaml -f /opt/prismkey/compose.nginx.yaml exec -T api python -m app.cli add-product --slug novel-words --name 词阅 --product-id NovelWords --issuer PrismKey_NovelWords --prefix CY
```

注册后在 PrismKey 管理台选择“词阅”生成卡密；客户端不包含管理账号、发卡凭证、pepper 或服务端签名私钥。产品目录只注册一次，不修改现有 XHS 产品及卡密。

生产注册已于 2026-09-17 完成：核对服务器 TLS 证书（Let's Encrypt 签发时间与 2026-09-15 新建时间吻合）与健康检查后更新了本机 known_hosts，随后执行 add-product，并验证 `GET /api/v1/public-key?product=novel-words` 返回 issuer `PrismKey_NovelWords`、audience `NovelWords`、kid `4906a9a5e11eef9e`。首批生产卡密已通过管理 API 生成。API 层真实卡密验收已完成（2026-09-17）：激活路由 `NovelWords`、Ed25519 严格验签与全部 claims、幂等重激活起点不变、换设备 409、停用后 403 LICENSE_DISABLED 均符合预期；测试卡已解绑停用并补发。原生客户端完整激活验收仍待执行。

## 缓存与设备

授权文件位于应用数据目录的 `license/`，独立于 SQLite 学习数据库，不随个人数据备份导出或恢复。Windows 使用当前用户的 DPAPI 加密，并以设备摘要作额外熵。Windows 设备摘要由产品标识和 MachineGuid 派生；Linux 优先使用 machine-id，无法获取系统标识的系统采用持久安装 ID。安装 ID 被清除会被视为不同设备。

非 Windows 使用权限受限的 AES-GCM 缓存与本地随机密钥，不宣称具有 Windows DPAPI 的账户隔离能力。此轮实际原生验收在 Windows 完成。

缓存读写失败时锁定授权，成功联网校验或重新激活后可恢复。授权模块初始化失败也保留受限页面与备份入口；修复编译配置或损坏的设备身份文件后需要重启。明确拒绝会清除签名凭证并持久保存拒绝结果；若整个缓存目录同时无法写入和删除，客户端不能保证该拒绝已持久化，此时会报告缓存失败且锁定当前进程，需要恢复存储后联网校验。

## 构建配置

以下变量在 Rust 编译时读取；它们只包含服务地址、产品身份及公钥，不能用来配置私钥：

- `NOVEL_WORDS_LICENSE_URL`
- `NOVEL_WORDS_LICENSE_PRODUCT_ID`
- `NOVEL_WORDS_LICENSE_ISSUER`
- `NOVEL_WORDS_LICENSE_CARD_PREFIX`
- `NOVEL_WORDS_LICENSE_PUBLIC_KEY_FILE`：Ed25519 SPKI PEM 公钥文件；未设置时使用内置生产公钥。

发布构建要求 HTTPS。仅调试构建允许 HTTP 回环服务。调试构建也执行真实签名验证，没有跳过卡密检查的开关。

## 隔离原生验证

测试直接启动用户提供项目的 PrismKey 后端，生成一次性密钥、pepper、SQLite 数据库和测试卡密。所有测试秘密只写到忽略的 `test-data/prismkey/`；不读取生产私钥、pepper 或管理员凭证。

使用该项目 `.venv/Scripts/python.exe`：

```powershell
$testSource = 'E:/Dsektop/项目资源/kami/PrismKey'
$testPython = "$testSource/.venv/Scripts/python.exe"
& $testPython tests/prepare-license-fixture.py prepare --source $testSource
& $testPython tests/prepare-license-fixture.py serve --source $testSource
```

在另一个终端创建 `license-smoke.local` 并启动 Vite、Tauri：

```json
{"identifier":"com.tauri-app.novel-words-license-test","build":{"beforeDevCommand":""}}
```

```powershell
npm run dev
```

```powershell
$env:NOVEL_WORDS_LICENSE_URL='http://127.0.0.1:18100'
$env:NOVEL_WORDS_LICENSE_PUBLIC_KEY_FILE=(Resolve-Path 'test-data/prismkey/test-public.pem').Path
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9229'
npm run tauri dev -- --config license-smoke.local
```

测试脚本拒绝正式应用数据目录：

```powershell
$env:PRISMKEY_TEST_SOURCE='E:/Dsektop/项目资源/kami/PrismKey'
$env:PRISMKEY_TEST_PYTHON="$env:PRISMKEY_TEST_SOURCE/.venv/Scripts/python.exe"
node tests/native-license.mjs core
# 关闭并重启同一隔离 Tauri 应用；此时测试服务保持模拟故障。
node tests/native-license.mjs restart
node tests/native-license.mjs lifecycle
```

测试输出不打印完整卡密。`core` 验证受保护命令、产品与设备隔离、加密缓存、预设整套导入、阅读查词收词、复习、备份恢复、PDF 和失败换卡；`restart` 验证离线重启期限及学习数据；`lifecycle` 验证停用、故障不复活、启用、解绑、再次激活、到期、深色英文界面及永久套餐。

测试服务可通过 `outage` / `online` 模拟 503。真正断网重启测试需先停止测试服务，并为 `restart` 设置 `NATIVE_EXPECT_NETWORK_DOWN=1`；完成后重新启动服务，再执行 `lifecycle`。原生页面首次加载较慢时，脚本等待 DOM 与操作目标，避免只以资源全部加载的事件判断就绪。

## 本轮验证结果（2026-09-17）

- `npm test`：40 项通过。
- `npm run test:ui`：41 项通过，含浅色、深色、中英文、800×500 和窄屏、键盘操作及锁定保护。
- `npm run build`：类型检查与生产前端构建通过。
- `cargo test --offline`：50 项通过，另主程序与文档测试通过。
- 错误私钥类型配置：实际 Cargo 构建在生成公钥常量前拒绝。
- 隔离原生测试：初始化故障仍可备份、产品及设备绑定、加密缓存、六级整套导入、阅读查词收词、复习、备份恢复、失败换卡与 462,619 字节 Rust PDF 已验证。
- 离线：503 固定凭证期限、停止 API 后真实断网重启、稳定设备与学习数据恢复已验证。
- 授权生命周期：停用后不能离线复活、恢复、解绑后显式激活并保留原套餐期限、已打开编辑内容保存、到期锁定、深色英文激活页及永久套餐已验证。

首轮脚本的页面等待及离线对比断言已修正；`core-finish` 仅允许在核心学习流程完整结束、已有 PDF 且统计匹配时补验最后的离线检查。生产产品注册与 API 层真实卡密验收已完成（2026-09-17）；原生客户端使用生产卡密的完整激活验收仍待执行。
