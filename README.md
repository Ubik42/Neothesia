<p align="center">
  <img src="assets/banner.png" alt="Neothesia" width="760">
</p>

# Neothesia

面向本地钢琴练习的跨平台 MIDI 可视化与训练工具。本仓库是在开源项目 [PolyMeilex/Neothesia](https://github.com/PolyMeilex/Neothesia) 基础上长期维护的个人分支，重点不只是播放“瀑布流”，而是把选曲、分段练习、演奏反馈、乐谱跟随和 Pianoteq 音源连接成一套稳定的日常练琴流程。

> **当前状态：暂停开发。** 功能停留在 2026-08-15 的 Cycle 105 检查点。现有练习功能可以从源码运行；Pianoteq VST3 直连仍属于进行中的 Windows 实现，不应视为已经打包完成的正式版本。

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

需要 Rust 工具链以及各平台对应的音频/MIDI 运行环境。

```powershell
git clone https://github.com/Ubik42/Neothesia.git
cd Neothesia
cargo run --release
```

常用验证：

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

脚本可以建立带来源和许可证记录的本地曲库。当前流程覆盖 MAESTRO、Mutopia 与受控数量的 Pop-K 条目；这些来源的许可条件并不相同，尤其部分数据集包含非商业限制。具体说明见[公开练习曲库](docs/pages/practice-library.md)。

## 当前边界

- 项目尚未提供本分支的正式安装包或自动更新流程。
- 刻谱渲染默认关闭，并依赖 Node/Verovio worker。
- Pianoteq VST3 已验证加载、实时 MIDI、音频和状态往返，但完整 UI、重启恢复、长时间稳定性和广泛插件兼容性仍需人工验收。
- 物理 MIDI 设备掉线重连、无障碍细节和大曲库渐进索引仍在路线图中。

## 工程结构

| 目录 | 职责 |
| --- | --- |
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
