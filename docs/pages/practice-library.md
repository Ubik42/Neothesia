# Public practice library

Neothesia does not bundle third-party repertoire. The helper script
`scripts/sync-practice-library.ps1` builds a local, source-aware practice and
test library under `D:\Music\MIDI\PracticeLibrary` by default.

## Included sources

| Collection | Purpose | Local selection | License handling |
| --- | --- | --- | --- |
| MAESTRO v3.0.0 | Classical performance MIDI | Complete 1,276-file MIDI release | CC BY-NC-SA 4.0; downloaded locally and checksum verified |
| Mutopia Project | Classical teaching and public-domain repertoire | 521 piano-focused files from composer and technique collections in the current sync | Per-piece license retained in `catalog.csv` |
| Pop-K v1.0 | Modern pop-style melody examples | First 256 numbered excerpts by default | CC BY-NC 4.0; downloaded locally and checksum verified |
| GiantMIDI-Piano v1.2 | Automatic transcriptions of classical piano performances | Full official release: 10,855 MIDI files, imported 2026-10-01 | Upstream states CC BY 4.0; disclaimer retained; unreviewed transcription category |

These sources are useful for private practice and development coverage, but
their terms are not identical. In particular, the MAESTRO and Pop-K datasets
have non-commercial restrictions. Do not redistribute the downloaded corpus as
part of a Neothesia binary or release.

## Run the sync

From PowerShell at the repository root:

```powershell
.\scripts\sync-practice-library.ps1
```

Use `-PopKCount` to change the bounded Pop-K subset, `-LibraryRoot` to select a
different local destination and `-Force` to re-download or re-expand existing
content. The script verifies official archive checksums before extraction.

The resulting `catalog.csv` records category, title, composer, license, source,
source URL and a library-relative path. Downloaded archives and MIDI files stay
outside Git. Mutopia links can disappear over time; an unavailable individual
piece is reported and skipped without invalidating the rest of the catalogue.

## Product integration status

The native Practice Library scans `catalog.csv` beside watched MIDI folders in
the same background indexing task. Catalogue title, composer, teaching
category, source and license are searchable and appear as read-only provenance
in the song list and information header. User-edited sidecar metadata remains
authoritative and is never overwritten by catalogue data.

Catalogue paths are treated as untrusted input: absolute paths and parent
traversal are rejected, both Windows and portable separators are accepted, and
one malformed row cannot prevent valid songs from loading. Run the real-corpus
inspection without opening the GPU interface with:

```powershell
cargo run -q -p neothesia-core --example library-inspect -- `
  "D:\Music\MIDI\PracticeLibrary" "Bach Mutopia"
```

## GiantMIDI-Piano 导入（2026-10-01）

参考源码已从用户的 fork 拉入 `D:\Music\_tools\_reference\GiantMIDI-Piano`。
仓库的 `midis_preview` 只有 4 首；`midis_for_evaluation` 是评估材料，不作为曲库。
完整 v1.2 压缩包从[上游免责声明中的官方公开链接](https://github.com/bytedance/GiantMIDI-Piano/blob/master/disclaimer.md)下载。
本机已导入完整 10,855 首，曲库现在共有 12,908 条来源记录。

```powershell
# 下载并准备完整 v1.2；已有压缩包时直接校验和解压。
python .\scripts\prepare-giantmidi-release.py
# 无需重新下载 MAESTRO、Pop-K 或抓取 Mutopia。
.\scripts\import-giantmidi-library.ps1 -MidiRoot "D:\Music\MIDI\projects\GiantMIDI-Piano\source"
```

也可以用 `-LibraryRoot` 指定曲库目录。准备工具支持 `--archive`、`--destination`；
只导入仓库的 4 首预览时，省略导入工具的 `-MidiRoot`。
常规同步会保留独立导入来源；需要一起导入时使用
`sync-practice-library.ps1 -IncludeGiantMidi -GiantMidiRoot <已准备的目录>`。

准备工具校验 ZIP 成员 CRC，并把 Windows 不支持的字符、过长文件名和仅大小写不同的
文件名转换成稳定名称；`filename-map.csv` 保留原始名称，导入时仍可正确关联曲名和作者。
上游资源文件扩展名是 `.csv`，实际以制表符分隔；导入按完整原始文件名匹配元数据。

压缩包 SHA-256 固定为
`41549405bcaeed4783e366f61236db4203c9b5d846fd8e0fee59bcf2658a23b7`。
这是 2026-10-01 从官方链接下载后计算的复现校验值，**不是上游公布的校验值**。
下载包缓存于曲库 `_archives`，原始准备目录在 `MIDI\projects`，应用扫描的副本在
`Classical_Transcription_GiantMIDI`；`last-import.json` 记录源码版本和逐文件 SHA-256。

导入会检查 MIDI 头部，遇到无效文件时跳过；同路径不同内容时拒绝覆盖。
重复导入按路径更新该来源记录，其他来源保留，目录清单以临时文件原子替换。
它不改写音符、量化节奏或自动分手，统一标为 `Classical transcription/GiantMIDI (unreviewed)`。
[上游说明](https://github.com/bytedance/GiantMIDI-Piano)明确这些 MIDI 来自演奏录音自动转录；
准确率、左右手、节拍网格、踏板和指法仍需核对，不能当成经过校对的教学谱。


### 精细查找与键盘选择

搜索可限定曲名、作曲家、演奏者、曲集、标签、难度、备注、路径或来源，默认全部信息。多个词同时匹配，双引号保留短语，前置减号排除，例如 `"练习曲" -"左手"`。可与分组/文件状态、最低评级、时长和练习记录筛选叠加。时长以分钟填写，只有已索引文件参与比较。同内容其他路径在索引识别后共用成绩，未识别不推断身份。每曲最多保留 200 次成绩，记录次数是保留数量；最近练习按实际保存成绩日期，单纯打开不影响排序。

用排序下拉及升/降序，或点曲名/作曲家/难度/时长/评级标题；未知值始终最后。查询自动保存，清除筛选恢复默认；分组、页码和临时选择不随重启恢复。

Enter 打开、空格勾选、方向键移动，Shift 连选可反向收缩；PageUp/PageDown 跨页，Ctrl+Home/End 到全部结果首尾。Ctrl+A 和选择全部结果包括全部筛选结果，表头只选本页。筛选隐藏的已选曲目仍保留；批量操作先看已选数量，取消选择全部清除。


### 文件变化自动更新

曲库管理的“自动更新”默认启用，开关在本机保存。登记的文件夹会自动发现新增 MIDI/midi，已有索引会检查 MIDI 和个人附加信息的变化。文件写入稳定后补索引；后台更新不会切换正在练习的曲目、跳动当前音符或改变计划条件。暂停自动更新后新增文件暂不出现；“立即检查”仍会完成一次发现和稳定确认，不会重新打开自动更新。

断开的文件夹保留先前目录，显示无法读取原因，曲目可在文件缺失筛选查看；接回目录后重新检查。移除登记停止目录发现，原文件、已有索引和成绩保留。更新采用定期检查，因此文件出现后需要短暂等待；写入持续变化的文件继续等待稳定。

同内容的新路径在解析确认后可以恢复分组和评级，并补回缺少的个人附加信息；新位置已有附加信息优先，冲突/损坏会提示而不会强行覆盖。收藏、历史和日课会查找已索引的新路径，再核验实际 MIDI 内容。当前已加载曲目仍使用原练习数据，需要明确重新打开才能切到新文件；不会将同名的新曲目误当作原曲。

自动目录发现目前处理 MIDI；MusicXML/MXL 仍用导入曲目打开。原乐谱更新后的重新生成、附件变更通知及缺失文件集中修复继续开发。

### 曲目谱面版本

选中已索引曲目，右侧直接列出全部演奏乐谱和纸面谱页。可以跨曲目改名、解除/设置关联、选择常用谱页、移除登记及预览 PDF/图片；只有打开指定谱面练习才切换当前曲目。表头选择只处理本页，不修改自动更新。详细规则见 [在曲库管理谱面版本](library-score-versions.md)。
