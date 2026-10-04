# Neothesia Web＋Tauri 练习室

新的中文主界面使用 React、TypeScript、Vite 与 Tauri 2。独立的
`neothesia-engine` Rust 工作线程负责播放时钟、MIDI 输入输出、内置钢琴音源、
跟弹匹配和练习记录；界面只发送命令和读取状态。

当前开发版：**Cycle 203（2026-10-03）**。本地曲目/多版本谱面管理、PDF/图片阅读与批注、指法推荐及双手审阅、练习计划/段落、历史演奏回放、曲目包与练习备份持续完善。最近三轮加入双手持音交接检查与共同慢练、指法方案逐音比较/局部采用，以及带实际文件副本的 `.neoscorebatch` 整理批次迁移。

具体完成范围与未完成项见 [产品路线](../docs/development/product-roadmap.md) 和 [逐轮开发进度](../docs/development/progress.md)。下面保留早期迁入能力说明；不能用早期批次列表代替当前完整功能对照。

Cycle 115：小节拖选/缩放分页/跟随、实际拍位定位/逐拍移动、完整演奏与背谱、完成后再演奏和薄弱段重练。

Cycle 114：可移植曲目包导出、导入预览、三种冲突处理，以及完整教学标注/谱面版本/分组评级/练习段落恢复。

Cycle 113：直接 MusicXML/MXL 练习、混合批量/拖放导入及可暂停/重启恢复的后台曲库索引已接入。

Cycle 112：逐音分手审阅/保存/撤销、按手生成指法、每手默认跨度、整曲拍号/弱起校正、多谱面版本管理与历史比较范围隔离已接入。完整剩余功能见 [产品路线](../docs/development/product-roadmap.md)。

## 直接运行

仓库根目录双击 `Open-Desktop.cmd` 启动已构建的 Tauri 桌面开发版。
桌面程序内置前端资源，不需要启动浏览器服务器。首次未构建时，入口会先执行构建。

双击 `Open-Web.cmd` 启动浏览器开发版：后台启动本地音乐服务和前端开发服务，
然后打开 `http://127.0.0.1:5173`。它与桌面版复用同一 Rust 引擎实现。
浏览器开发服务只监听回环地址，桌面版使用 Tauri 命令通信。

内置“从中央 C 开始”的九音五指热身，因此没有外部曲库也可以练习。
外部曲库默认读取 `D:\Music\MIDI\PracticeLibrary\catalog.csv`，
可用 `NEOTHESIA_LIBRARY_ROOT` 环境变量指定其他目录。
列表直接读取目录清单；只有选中的曲目才解析 MIDI。

## 早期已接入的基础能力（Cycle 115）

- 中文演奏工作台：可折叠曲库、键盘提示或乐谱、音轨与练习设置、固定播放控制。
- MIDI 拍号、变拍号与速度图；显示小节和拍，按小节导航、选段及循环。6/8 显示两大拍；缺失拍号明确标识为默认值。
- 等音、连续、聆听三种模式；逐音轨练习、伴奏、静音与隐藏。左右手使用声部归属，未知声部可手动指定。
- 有声预备拍、节拍器、循环遍数、独立逐轮成绩、可选自适应速度；每曲设置独立恢复。
- 结构化小节、声部、和弦、时机、力度、踏板和时值反馈；反馈可直接重练段落，历史按模式和声部区分。
- 完整技术练习生成器：调性、大小调与小调形式、音阶/琶音/和弦、手别、方向、八度、遍数及速度；带已有教学指法。
- MusicXML/MXL 配对、真实 Verovio WASM 排版、对齐诊断、可靠音符定位和翻页跟随；支持手动指法编辑。
- 自由演奏录音、MIDI 保存、回放和导出。录音不会覆盖原曲练习模式。
- 收藏、最近、队列与排序；桌面 MIDI/文件夹导入、元数据编辑、内容身份校验与缺失路径重新定位。
- 原生 MIDI/SoundFont、输入重连、紧急停止、设备独立延迟校准；复用原生 VST3 插件发现、编辑器及状态存储后端。
- 公共曲库 12,908 首，加五组原创入门练习。GiantMIDI 自动转录仍标记待校对。

桌面记录保存在 Tauri 用户应用数据目录；浏览器开发版默认使用仓库 `work/web-data`，可用 `NEOTHESIA_DATA_ROOT` 覆盖。旧原生程序和历史保留，没有自动合并。

核心共享了原生音轨配置、匹配器、历史、录音器与 VST3 后端。完整功能对照见 [当前产品路线](../docs/development/product-roadmap.md)：指法审阅、网格编辑、曲库整理、文件变化检查和备份已持续扩展；各能力的具体范围与余项以该路线为准。VST3 编辑器兼容性、真实实弹及持续断连恢复仍保留原先实测边界；打开 MIDI 端口不等于完成实弹验收。

## 开发与构建

```powershell
npm ci
npm run dev
# 另一个后台进程，在仓库根目录运行
cargo run -p neothesia-engine --bin neothesia-service
# 快速生成包含前端资源的桌面开发版
npm run desktop:build:dev
# 优化构建
npm run desktop:build
```

`npm run desktop` 是 Tauri 开发模式，会启动真实桌面窗口；日常前端开发和验证
优先使用浏览器无界面模式。

## 验证

```powershell
cargo test -p neothesia-engine --lib
# 初始化真实音频设备和 SoundFont，不发出音符
cargo run -p neothesia-engine --bin neothesia-service -- --audio-probe
```

浏览器端运行 `npm test`。需要先启动前端和 Rust 本地服务；自动化服务用 `--silent`
避免测试期间发声，并设置独立的 `NEOTHESIA_DATA_ROOT`。测试通过真实后端完成
等待、暂停、页面刷新、九音演奏和持久化，不使用模拟后端。
第二项测试读取实际 GiantMIDI 曲库并检查来源提示与窄屏溢出。

本地开发服务协议：`GET /api/library`、`GET /api/state`、`GET /api/song`、
`GET /api/devices`、`POST /api/command`。写请求需要客户端标识，浏览器来源限定为
本地前端地址；曲目加载只允许目录清单登记的路径。桌面导入通过系统文件选择器授权。

Tauri 隐藏窗口检查可设置 `NEOTHESIA_HEADLESS_CHECK=1`；实际桌面启动默认显示窗口。
此变量只用于验证窗口生命周期，不会把硬件验证变成无界面测试。
也可运行 `Neothesia.exe --headless-check`：等待真实前端 DOM 与引擎连接，
通过 Tauri IPC 读取内置 MIDI 并初始化音色和音频设备，成功后返回退出码 0。
检查不打开远程调试端口，也不播放音符。

运行包可通过仓库的 `scripts/package-web-practice.ps1 -Destination <目录>` 重建，
包含 `Neothesia.exe`、`default.sf2` 和中文使用说明。

设置存储在同一数据目录的 `web-preferences.json`，通过临时文件替换保存。
循环范围是 `[起点, 终点)`；终点上的音符归下一段。设置范围会停止当前播放，
点击开始练习后连续重复。跳转需位于当前范围内，或先关闭循环。
每轮清空本轮统计，历史保留各轮结果。设备缺失时给出提示，输入重连不会自动演奏。

Windows 启动脚本使用 ASCII 内容和 CRLF 换行，避免中文文件夹名的命令行编码问题。
便携包双击 `Neothesia.exe`，或使用同目录的 `Open.cmd`。桌面数据目录有进程锁，
避免重复启动同时覆盖练习历史。启动失败会弹出错误并写入临时目录的
`neothesia-startup-error.txt`。`--window-check` 会验证真实可见窗口、DOM、IPC、
曲库、MIDI 和真实音频初始化后退出；验证应使用独立 `NEOTHESIA_DATA_ROOT`。
