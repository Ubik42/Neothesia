<p align="center">
  <img src="assets/banner.png" alt="Neothesia" width="760">
</p>

# Neothesia

面向本地钢琴练习的跨平台 MIDI 可视化与训练工具。本仓库是在开源项目 [PolyMeilex/Neothesia](https://github.com/PolyMeilex/Neothesia) 基础上长期维护的个人分支，把选曲、分段练习、演奏反馈、乐谱跟随和 Pianoteq 音源连接成一套稳定的日常练琴流程。

> **当前开发版：Cycle 203（2026-10-03）。** React＋Tauri 中文练习室与共享 Rust 引擎已接入本地曲库整理、多版本 MusicXML/PDF/图片谱面、指法推荐与逐音审阅、双手交接检查、练习段落与计划、演奏记录和备份迁移。最近新增谱面区域/笔迹批注、批量配对的文件副本与重启恢复、指法方案局部比较采用，以及 `.neoscorebatch` 整理批次导出/导入。具体完成范围、验证和未完成项见 [开发进度](docs/development/progress.md) 与 [产品路线](docs/development/product-roadmap.md)。这是持续开发的源码快照，尚未完成全部 Synthesia 功能。

## 新练习室入口

双击根目录 **`Open-Desktop.cmd`** 启动新版 Tauri 桌面开发版；
**`Open-Web.cmd`** 启动浏览器开发版和后台音乐引擎。
新版使用 React＋TypeScript 中文界面，Rust 独立线程负责播放、设备和评分，
并已支持无界面浏览器操作真实后端进行自动验证。
迁移范围、构建、数据目录和当前边界见 [Web＋Tauri 练习室](neothesia-web/README.md)。

下文“已实现能力”主要描述旧原生分支，不能视为所有能力均已迁入新版。

![Neothesia playback interface](https://github.com/PolyMeilex/Neothesia/assets/20758186/65483bab-0b74-4fd4-90b1-fdd00508b676)

## 这个分支解决什么问题

普通 MIDI 可视化器适合跟弹，却很难回答“这一段为什么总弹不好、下次应该练什么”。这个分支把练习过程拆成可恢复、可度量的闭环：

```text
本地曲库 → 选择段落与左右手 → 等待正确音符 / 自适应速度
        → 音准、节奏、力度、时值、踏板反馈 → 保存练习记录与建议
```

所有曲库、设置与练习记录保留在本地。模型、账号和云端服务都不是日常练习的前置条件。

## 已实现能力

- **专注练习**：等待音符模式、左右手筛选、小节循环、预备拍、节拍细分与自适应速度。
- **可解释反馈**：分别统计音准、时机、左右手、小节、力度、时值与踏板表现，并保存练习会话和改进建议。
- **本地曲库**：监视多个文件夹，支持搜索、最近曲目、收藏、练习队列、元数据编辑、来源与许可证记录，以及缺失文件修复。
- **MIDI 与乐谱配对**：可关联 MusicXML/MXL，使用可选的 Verovio 渲染乐谱，并在播放时翻页、定位和高亮当前音符。
- **指法与自由演奏**：支持手动/建议指法、练习模式以及自由演奏录制。
- **外部音源**：可把实时 MIDI 安全地路由到独立运行的 Pianoteq，包含暂停、跳转和退出时的 all-notes-off / pedal-up 保护。
- **实验性 VST3 托管**：Windows 下能够发现并加载 Pianoteq VST3、发送实时 MIDI 和输出音频；编辑器嵌入、重启后的状态恢复和第三方兼容性仍待完整验证。

## 乐谱与练习界面

[![Neothesia demonstration video](https://github.com/PolyMeilex/Neothesia/assets/20758186/dc564433-aade-4430-b137-5f90000ae9e0)](https://youtu.be/ReE9nVuMCSE)

上面的公开视频展示上游项目的基础播放界面。本分支新增的刻谱视图、语义高亮、曲库来源信息和 Pianoteq 工作流以代码、自动化检查与开发记录为准，详见[开发进度](docs/development/progress.md)。

## 从源码运行

新版桌面入口需要 Rust 和 Node.js/npm，以及平台对应的音频/MIDI 运行环境；Windows 使用 WebView2。

```powershell
git clone https://github.com/Ubik42/Neothesia.git
cd Neothesia
cd neothesia-web
npm ci
npm run desktop:build:dev
cd ..
.\Open-Desktop.cmd
```

旧原生界面仍可从仓库根目录运行 `cargo run --release`。新版浏览器开发、桌面打包与针对性验证见 [练习室开发说明](neothesia-web/README.md)。

核心验证：

```powershell
cargo test
cargo clippy --all-targets
```

刻谱功能使用可选的 Node/Verovio worker；Pianoteq 既可以通过虚拟 MIDI 端口独立运行，也可以在 Windows 上试用当前的 VST3 托管实现。配置与边界见：

- [MusicXML 与刻谱架构](docs/pages/musicxml-architecture.md)
- [外部 Pianoteq 路由](docs/pages/pianoteq-external-routing.md)
- [VST3 托管路线图](docs/pages/plugin-hosting-roadmap.md)

## 本地练习曲库

仓库提供同步脚本，但不把第三方曲库打包进发行版：

```powershell
.\scripts\sync-practice-library.ps1
```

脚本可以建立带来源和许可证记录的本地曲库。当前流程覆盖 MAESTRO、Mutopia、受控数量的 Pop-K 条目，以及独立导入的 GiantMIDI-Piano v1.2（10,855 首）。GiantMIDI 是自动转录的演奏数据，归入待校对分区；这些来源的许可条件并不相同，尤其部分数据集包含非商业限制。完整下载、导入与来源说明见[公开练习曲库](docs/pages/practice-library.md)。

## 当前边界

- 当前为开发版；正式安装器、自动更新和完整跨平台验收仍未完成。
- 新版使用随源码提供的 Verovio WASM；旧原生刻谱仍采用可选的 Node/Verovio worker。
- Pianoteq VST3 已验证加载、实时 MIDI、音频和状态往返，但完整 UI、重启恢复、长时间稳定性和广泛插件兼容性仍需人工验收。
- MIDI 重连和后台曲库索引已接入；复杂乐谱解释、专家指法校准、跨演奏身份资料迁移及完整产品验收继续推进，具体边界以产品路线为准。

## 工程结构

| 目录 | 职责 |
| --- | --- |
| `neothesia-web/` | 新版 React 中文界面与 Tauri 桌面壳 |
| `neothesia-engine/` | 独立播放线程、设备、音频、评分和浏览器开发服务 |
| `neothesia/` | 桌面应用、场景与交互 |
| `neothesia-core/` | 练习、曲库、乐谱、反馈与渲染核心 |
| `midi-file/` / `midi-io/` | MIDI 文件播放与设备输入输出 |
| `piano-layout/` | 键盘布局 |
| `docs/` | 使用说明、架构与逐周期开发证据 |
| `scripts/` | 曲库同步、兼容性检查与练习流程验证 |

## 开发文档

- [当前开发状态](docs/development/progress.md)
- [开发文档入口](docs/development/README.md)
- [路线图](docs/development/roadmap.md)
- [有序 Backlog](docs/development/backlog.md)
- [快捷键](docs/pages/shortcuts.md)
- [指法建议](docs/pages/fingering-suggestions.md)

## 上游与许可证

本项目延续 [PolyMeilex/Neothesia](https://github.com/PolyMeilex/Neothesia) 的开源工作，并使用 WGPU、Linthesia、Synthesia 等项目提供的技术与设计参考。许可证见 [LICENSE](LICENSE)。
