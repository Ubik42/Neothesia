import {
  encodeWorkspaceBackup,
  decodeWorkspaceBackup,
} from "./workspaceBackup";
import { downloadWorkspaceBackup } from "./workspaceDownload";
import {
  listBatches,
  loadBatch,
  saveImportedBatches,
  type BatchData,
  type BatchSummary,
} from "./batchImportStore";
import "./workspaceBackupDialog.css";
import { templateFields, modes, type Rules } from "./metadataTemplates";
import { useRef, useState, useEffect } from "react";
import { X, Download, FolderOpen } from "lucide-react";
import { api } from "./api";
import { scheduleText, type RoutineSchedule } from "./RoutineScheduleEditor";
type Summary = {
  timeLedgerEntries?: number;
  timeLedgerRunningMs?: number;
  timeLedgerUntrackedMs?: number;
  timeLedgerDuplicates?: number;
  timeLedgerConflicts?: number;
  replayClips?: number;
  replayClipConflicts?: number;
  performances?: number;
  performanceBytes?: number;
  performanceConflicts?: number;
  duplicatePerformances?: number;
  performanceErrors?: { contentId: string; id: string; error: string }[];
  metadataTemplates?: number;
  metadataHistory?: number;
  metadataHistorySongs?: number;
  metadataHistoryAdded?: number;
  metadataHistoryDuplicates?: number;
  metadataHistoryUnmatched?: number;
  metadataHistoryAmbiguous?: number;
  metadataHistoryPendingAdded?: number;
  metadataHistoryError?: string | null;
  metadataTemplateRows?: { name: string; rules: Partial<Rules> }[];
  templateError?: string | null;
  learning?: number;
  learningConflicts?: number;
  token: string;
  createdAt: number;
  plans: {
    id: string;
    name: string;
    notes: string;
    schedule?: RoutineSchedule | null;
    items: number;
  }[];
  days: number;
  songs: number;
  records: number;
  passages: number;
  ladders: number;
  exercises: number;
  trackSounds?: number;
  trackAppearances?: number;
  conflicts: number;
  dayConflicts: number;
  presetConflicts: number;
  conflictRows: { kind: string; local: string; incoming: string }[];
  duplicateRecords: number;
  annotationConflicts: number;
  references: {
    contentId: string;
    title: string;
    generated: boolean;
    indexed: boolean;
  }[];
};
function timeText(value: number) {
  const ms = Math.max(0, Math.floor(value));
  const seconds = Math.floor(ms / 1000) % 60;
  return `${Math.floor(ms / 3600000)} 小时 ${Math.floor(ms / 60000) % 60} 分 ${seconds}${ms % 1000 ? `.${String(ms % 1000).padStart(3, "0")}` : ""} 秒`;
}
export function PracticeBackupDialog({
  close,
  changed,
}: {
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [selection, setSelection] = useState({
      plans: true,
      records: true,
      presets: true,
      days: 0,
    }),
    [summary, setSummary] = useState<Summary | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState(""),
    [policy, setPolicy] = useState("keep");
  const [localBatches, setLocalBatches] = useState<BatchSummary[]>([]);
  const [batchError, setBatchError] = useState("");
  const [progress, setProgress] = useState("");
  const [pendingHistory, setPendingHistory] = useState<{sourcePath:string;contentId:string;title?:string|null;records:number;candidates:string[]}[]>([]);
  const [pendingTargets, setPendingTargets] = useState<Record<string,string>>({});
  const [pendingError, setPendingError] = useState("");
  const [workspace, setWorkspace] = useState<{
    practice: File;
    batches: BatchData[];
    selected: string[];
    saved: number;
  } | null>(null);
  const batchSaved = useRef(false);
  const workspaceInput = useRef<HTMLInputElement>(null);
  const input = useRef<HTMLInputElement>(null),
    working = useRef(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    void refreshPending();
    void listBatches()
      .then((rows) => {
        if (alive.current) setLocalBatches(rows);
      })
      .catch((e) => {
        if (alive.current) setBatchError(String(e));
      });
    return () => {
      alive.current = false;
    };
  }, []);
  const refreshPending = async () => {
    try {
      const result = await api.command<{rows:typeof pendingHistory}>({type:"pendingMetadataHistory"});
      if (alive.current) { setPendingHistory(result.rows); setPendingError(""); }
    } catch (e) { if (alive.current) setPendingError(String(e)); }
  };
  const claimPending = (row: typeof pendingHistory[number]) => work(async () => {
    const key = `${row.contentId}:${row.sourcePath}`;
    const target = pendingTargets[key] || (row.candidates.length === 1 ? row.candidates[0] : "");
    if (!target) throw new Error("请先选择要接收资料修改历史的曲目");
    const result = await api.command<{added:number;records:number}>({type:"claimPendingMetadataHistory",source_path:row.sourcePath,content_id:row.contentId,target_path:target});
    setMessage(`已将 ${result.records} 条修改历史认领到所选曲目，新增 ${result.added} 条。`);
    await refreshPending();
    await changed();
  });
  const work = async (job: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    setMessage("");
    setProgress("");
    try {
      await job();
    } catch (e) {
      if (alive.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  };
  const pick = () => {
    if (api.desktop)
      void work(async () => {
        setWorkspace(null);
        batchSaved.current = false;
        setSummary(null);
        const data = await api.pickPracticeBackup();
        if (data) setSummary(data as Summary);
      });
    else input.current?.click();
  };
  const exportFile = () =>
    void work(async () => {
      if (api.desktop) {
        const path = await api.exportPracticeBackup(selection);
        if (path) setMessage(`备份已保存：${path}`);
      } else {
        const blob = await api.downloadPracticeBackup(selection);
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `Neothesia-练习资料-${new Date().toISOString().slice(0, 10)}.neopractice`;
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        setMessage("练习资料备份已导出");
      }
    });
  const reportProgress = (text: string) => {
    if (alive.current) setProgress(text);
  };
  const exportWorkspace = () =>
    void work(async () => {
      reportProgress("正在读取已保存整理批次…");
      const listed = await listBatches();
      const batches: BatchData[] = [];
      for (const [index, row] of listed.entries()) {
        reportProgress(
          `正在读取批次 ${index + 1}/${listed.length}：${row.name}`,
        );
        const value = await loadBatch(row.id);
        if (value.revision !== row.revision)
          throw new Error("整理批次在导出期间发生了变化，请重新导出。");
        batches.push(value);
      }
      const after = await listBatches();
      if (
        JSON.stringify(after.map((b) => [b.id, b.revision]).sort()) !==
        JSON.stringify(listed.map((b) => [b.id, b.revision]).sort())
      )
        throw new Error("整理批次在导出期间发生了变化，请重新导出。");
      setLocalBatches(after);
      setBatchError("");
      reportProgress("正在导出所选练习资料…");
      const practice = await api.downloadPracticeBackup(selection);
      const file = await encodeWorkspaceBackup(
        practice,
        batches,
        reportProgress,
      );
      const path = await downloadWorkspaceBackup(
        file,
        `Neothesia-完整资料-${new Date().toISOString().slice(0, 10)}.neoworkspace`,
        reportProgress,
      );
      setMessage(
        path === null
          ? "本次完整资料备份未保存。"
          : `完整资料备份已导出：${batches.length} 个整理批次、${batches.reduce((n, b) => n + b.items.length, 0)} 个文件。${path ? `保存位置：${path}` : ""}`,
      );
    });
  const importFile = () =>
    void work(async () => {
      if (!summary) return;
      let savedCount = workspace?.saved ?? 0;
      if (workspace && !batchSaved.current) {
        const batches = workspace.batches.filter((b) =>
          workspace.selected.includes(b.id),
        );
        if (batches.length) {
          reportProgress(`正在保存 ${batches.length} 个整理批次及原始文件…`);
          await saveImportedBatches(batches);
          savedCount = batches.length;
          batchSaved.current = true;
          setWorkspace({ ...workspace, saved: savedCount });
          // The committed batch flag is recorded before importing practice data so retries cannot duplicate files.
          void listBatches()
            .then((rows) => {
              if (alive.current) setLocalBatches(rows);
            })
            .catch(() => {});
        }
      }
      try {
        if (available) {
          reportProgress("正在合并所选练习资料…");
          const result = await api.command<{
            importedPerformances: number;
            replacedPerformances: number;
            keptPerformances: number;
            duplicatePerformances: number;
            trimmedPerformances: number;
            addedTimeEntries?: number;
            addedRecords: number;
            duplicateRecords: number;
            renamedRecords: number;
            trimmedRecords: number;
            metadataHistoryAdded?: number;
            metadataHistoryDuplicates?: number;
            metadataHistoryUnmatched?: number;
            metadataHistoryAmbiguous?: number;
            metadataHistoryPendingAdded?: number;
          }>({
            type: "importStagedPracticeBackup",
            token: summary.token,
            selection: { ...selection, days: 0 },
            policy,
          });
          // A refresh failure must not invite importing already-committed data again.
          setSummary(null);
          setWorkspace(null);
          batchSaved.current = false;
          setMessage(
            `${savedCount ? `整理批次已保存 ${savedCount} 个。` : ""}导入完成：曲目资料历史新增 ${result.metadataHistoryAdded ?? 0} 条、跳过重复 ${result.metadataHistoryDuplicates ?? 0} 条、另存待认领 ${result.metadataHistoryPendingAdded ?? 0} 条。成绩新增 ${result.addedRecords} 条、跳过重复 ${result.duplicateRecords} 条、区分同编号记录 ${result.renamedRecords}${result.trimmedRecords ? `；每曲保留上限淘汰最早 ${result.trimmedRecords} 条` : ""}。回放新增 ${result.importedPerformances ?? 0} 次、恢复/替换 ${result.replacedPerformances ?? 0} 次、保留本地 ${result.keptPerformances ?? 0} 次、跳过重复 ${result.duplicatePerformances ?? 0} 次${result.trimmedPerformances ? `，随淘汰成绩跳过 ${result.trimmedPerformances} 次` : ""}。累计时长新增 ${result.addedTimeEntries ?? 0} 条。本机导入前副本已保留。`,
          );
          await refreshPending();
          try {
            await changed();
          } catch (e) {
            setError(
              `资料已导入，但页面刷新失败：${String(e)}。请关闭此窗口后重新打开。`,
            );
          }
        } else {
          setSummary(null);
          setWorkspace(null);
          batchSaved.current = false;
          setMessage(
            `导入完成：已保存 ${savedCount} 个整理批次及其原始文件。您可以从“导入曲目 → 批量配对谱面”继续整理。`,
          );
        }
      } catch (e) {
        if (savedCount)
          throw new Error(
            `整理批次已保存 ${savedCount} 个，现有批次保留。练习资料未完成导入：${e instanceof Error ? e.message : String(e)}。您可以排除原因后重试练习资料，系统会跳过已保存的批次。`,
          );
        throw e;
      }
    });
  const any = selection.plans || selection.records || selection.presets;
  const available =
    summary &&
    ((selection.plans &&
      (summary.plans.length || summary.days || summary.learning)) ||
      (selection.records && (summary.records || summary.timeLedgerEntries)) ||
      (selection.presets &&
        summary.passages +
          summary.ladders +
          summary.exercises +
          (summary.trackSounds ?? 0) +
          (summary.trackAppearances ?? 0) +
          (summary.metadataTemplates ?? 0) +
          (summary.metadataHistory ?? 0)));
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog practice-backup-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="practice-backup-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="practice-backup-title">练习资料备份</h2>
            <p>计划、要求、日期进度和个人成绩</p>
          </div>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭练习资料备份"
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <p className="parameter-help">
          备份按曲目内容身份关联。MIDI、谱面、指法、逐音分手及拍号修正请通过曲目包迁移；这里保留练习计划、日期安排、曲目学习资料、资料修改历史、片段、阶梯、资料模板、成绩历史和已有演奏回放。设备选择与设备校准保留本机设置。
        </p>
        <div className="practice-backup-selection">
          {[
            ["plans", "计划、日课与曲目学习"],
            ["records", "个人成绩与演奏回放"],
            ["presets", "片段、阶梯、技术练习、音轨、资料模板与修改历史"],
          ].map(([key, label]) => (
            <label key={key}>
              <input
                type="checkbox"
                disabled={busy}
                checked={selection[key as "plans" | "records" | "presets"]}
                onChange={(e) =>
                  setSelection((s) => ({ ...s, [key]: e.target.checked }))
                }
              />
              {label}
            </label>
          ))}
        </div>
        <section className="practice-backup-export" aria-label="待认领资料修改历史">
          <h3>待认领的资料修改历史</h3>
          <p className="parameter-help">备份中无法确定归属的记录会保存在本机。曲目入库后，请在这里选择内容相同的 MIDI 路径。选择前请核对来源与目标；认领只增加修改历史，不会改写曲目资料。</p>
          {pendingError && <p role="alert">待认领记录无法读取：{pendingError}</p>}
          {pendingHistory.length === 0 && !pendingError && <p className="parameter-help">目前没有待认领记录。</p>}
          {pendingHistory.map((row) => {
            const key = `${row.contentId}:${row.sourcePath}`;
            return <div key={key} className="practice-backup-pending-row">
              <p><strong>{row.title || "未命名曲目"}</strong> · {row.records} 条记录</p>
              <p className="parameter-help">原路径：{row.sourcePath}</p>
              <p className="parameter-help">内容身份：{row.contentId}</p>
              {row.candidates.length ? <><select aria-label={`认领目标 ${row.sourcePath}`} disabled={busy} value={pendingTargets[key] ?? (row.candidates.length === 1 ? row.candidates[0] : "")} onChange={(e)=>setPendingTargets((old)=>({...old,[key]:e.target.value}))}>
                {row.candidates.length > 1 && <option value="">选择目标 MIDI</option>}
                {row.candidates.map((path)=><option key={path} value={path}>{path}</option>)}
              </select><button type="button" disabled={busy || (row.candidates.length > 1 && !pendingTargets[key])} onClick={()=>void claimPending(row)}>认领到所选曲目</button></> : <p className="parameter-help">曲库中还没有内容相同的 MIDI。请先导入或索引曲目，再重新打开此窗口。</p>}
            </div>;
          })}
          <button type="button" disabled={busy} onClick={()=>void refreshPending()}>刷新待认领记录</button>
        </section>
        <div className="practice-backup-export">
          <h3>保存本机资料</h3>
          <p className="parameter-help">
            成绩日期范围只筛选单次成绩和演奏回放。勾选个人成绩时，备份同时保留全部累计练习时长，包含已超出单曲
            200 条成绩保留上限的运行时间。
          </p>
          <label>
            导出成绩范围
            <select
              aria-label="备份成绩日期范围"
              disabled={busy}
              value={selection.days}
              onChange={(e) =>
                setSelection((s) => ({ ...s, days: Number(e.target.value) }))
              }
            >
              <option value={0}>全部保留记录</option>
              <option value={30}>最近 30 天</option>
              <option value={90}>最近 90 天</option>
              <option value={365}>最近一年</option>
            </select>
          </label>
          <button disabled={busy || !any} onClick={exportFile}>
            <Download size={14} />
            导出练习备份
          </button>
        </div>
        <section className="workspace-backup-section" aria-label="完整资料备份">
          <h3>完整资料备份</h3>
          <p>
            完整资料文件会合并上方勾选的练习资料，以及全部已保存整理批次。每个整理批次携带原始文件、配对选择、图片页序和整理进度。
          </p>
          <p>
            这份文件覆盖所选练习资料和已保存整理批次的原文件副本。曲库内其他
            MIDI、谱面原件及各曲的指法和批注，需要保留原文件路径或另行导出曲目包。
          </p>
          <p>
            本机已保存 {localBatches.length} 个整理批次，共{" "}
            {localBatches.reduce((n, b) => n + b.total, 0)} 个文件，
            {(
              localBatches.reduce((n, b) => n + b.bytes, 0) /
              1024 /
              1024
            ).toFixed(1)}{" "}
            MB。
            {!localBatches.length &&
              "您尚未保存整理批次，完整资料备份将只包含所选练习资料。"}
          </p>
          {batchError && (
            <p role="alert">
              整理批次目录读取失败：{batchError}
              。您可以重试导出，系统会重新读取目录。
            </p>
          )}
          {!any && (
            <p>导出需要至少勾选一类练习资料。导入时，您可以只恢复整理批次。</p>
          )}
          <div className="workspace-backup-actions">
            <button disabled={busy || !any} onClick={exportWorkspace}>
              <Download size={14} />
              导出完整资料备份
            </button>
            <button
              disabled={busy}
              onClick={() => workspaceInput.current?.click()}
            >
              <FolderOpen size={14} />
              导入完整资料备份
            </button>
          </div>
          <input
            hidden
            ref={workspaceInput}
            type="file"
            accept=".neoworkspace"
            aria-label="完整资料备份文件"
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = "";
              if (file)
                void work(async () => {
                  setSummary(null);
                  setWorkspace(null);
                  batchSaved.current = false;
                  const decoded = await decodeWorkspaceBackup(
                    file,
                    reportProgress,
                  );
                  reportProgress("文件校验完成，正在核对练习资料…");
                  const staged = (await api.stagePracticeBackup(
                    decoded.practice,
                  )) as Summary;
                  setWorkspace({
                    ...decoded,
                    selected: decoded.batches.map((b) => b.id),
                    saved: 0,
                  });
                  setSummary(staged);
                  setMessage(
                    "备份已校验。请核对下方练习资料和整理批次，然后执行导入。",
                  );
                });
            }}
          />
        </section>
        <div className="practice-backup-import">
          <h3>导入已有备份</h3>
          <button disabled={busy} onClick={pick}>
            <FolderOpen size={14} />
            选择练习备份
          </button>
          <input
            hidden
            ref={input}
            type="file"
            accept=".neopractice"
            aria-label="练习备份文件"
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = "";
              if (file)
                void work(async () => {
                  setSummary(null);
                  setWorkspace(null);
                  batchSaved.current = false;
                  setSummary((await api.stagePracticeBackup(file)) as Summary);
                });
            }}
          />
          {summary && (
            <>
              {workspace && (
                <section
                  className="workspace-backup-preview"
                  aria-label="整理批次恢复预览"
                >
                  <h3>整理批次 · {workspace.batches.length} 个</h3>
                  <p>
                    恢复的批次会成为新的本机待核对版本，现有批次保留。同一份完整备份已经恢复过的批次会被拒绝重复保存；您可以取消勾选批次，只导入练习资料。
                  </p>
                  {!workspace.batches.length && <p>这份备份没有整理批次。</p>}
                  {!!workspace.saved && (
                    <p className="backup-message">
                      本次已保存 {workspace.saved}{" "}
                      个整理批次。后续重试只处理练习资料。
                    </p>
                  )}
                  <div className="workspace-batch-list">
                    {workspace.batches.map((batch) => (
                      <label key={batch.id}>
                        <input
                          type="checkbox"
                          aria-label={`恢复整理批次 ${batch.name}`}
                          checked={workspace.selected.includes(batch.id)}
                          disabled={busy || !!workspace.saved}
                          onChange={(e) =>
                            setWorkspace({
                              ...workspace,
                              selected: e.target.checked
                                ? [...workspace.selected, batch.id]
                                : workspace.selected.filter(
                                    (id) => id !== batch.id,
                                  ),
                            })
                          }
                        />
                        <span>
                          <strong>{batch.name}</strong>
                          <small>
                            {batch.items.length} 个文件 ·{" "}
                            {(
                              batch.items.reduce(
                                (n, item) => n + item.file.size,
                                0,
                              ) /
                              1024 /
                              1024
                            ).toFixed(1)}{" "}
                            MB · 需要核对{" "}
                            {
                              batch.items.filter((i) => i.state === "done")
                                .length
                            }{" "}
                            项
                          </small>
                        </span>
                      </label>
                    ))}
                  </div>
                  {!!workspace.saved && (
                    <button
                      disabled={busy}
                      onClick={() =>
                        void work(async () => {
                          setSummary(
                            (await api.stagePracticeBackup(
                              workspace.practice,
                            )) as Summary,
                          );
                          setMessage(
                            "练习资料已重新核对，已保存的整理批次保留。请核对后重试导入。",
                          );
                        })
                      }
                    >
                      重新核对练习资料
                    </button>
                  )}
                </section>
              )}
              <p>
                备份日期：{new Date(summary.createdAt).toLocaleString("zh-CN")}
              </p>
              <dl className="practice-backup-counts">
                <div>
                  <dt>累计时长条目 / 已有相同条目</dt>
                  <dd>
                    {summary.timeLedgerEntries ?? 0} /{" "}
                    {summary.timeLedgerDuplicates ?? 0}
                  </dd>
                </div>
                <div>
                  <dt>可按日期统计的运行时长</dt>
                  <dd>{timeText(summary.timeLedgerRunningMs ?? 0)}</dd>
                </div>
                <div>
                  <dt>日期不明的运行时长</dt>
                  <dd>{timeText(summary.timeLedgerUntrackedMs ?? 0)}</dd>
                </div>
                <div>
                  <dt>演奏回放 / 内容冲突</dt>
                  <dd>
                    {summary.performances ?? 0} /{" "}
                    {summary.performanceConflicts ?? 0}
                  </dd>
                </div>
                <div>
                  <dt>回放资料大小</dt>
                  <dd>
                    {((summary.performanceBytes ?? 0) / 1024 / 1024).toFixed(1)}{" "}
                    MB
                  </dd>
                </div>
                <div>
                  <dt>曲目学习资料 / 冲突</dt>
                  <dd>
                    {summary.learning ?? 0} / {summary.learningConflicts ?? 0}
                  </dd>
                </div>
                <div>
                  <dt>常用计划 / 日期安排</dt>
                  <dd>
                    {summary.plans.length} / {summary.days}
                  </dd>
                </div>
                <div>
                  <dt>曲目 / 成绩记录</dt>
                  <dd>
                    {summary.songs} / {summary.records}
                  </dd>
                </div>
                <div>
                  <dt>片段 / 阶梯 / 技术方案 / 资料模板</dt>
                  <dd>
                    {summary.passages} / {summary.ladders} / {summary.exercises}{" "}
                    / {summary.metadataTemplates ?? 0}
                    {summary.trackAppearances
                      ? ` · ${summary.trackAppearances} 个音轨名称/颜色`
                      : ""}
                    {summary.trackSounds
                      ? ` · ${summary.trackSounds} 个音轨声音设置`
                      : ""}
                  </dd>
                </div>
                <div>
                  <dt>曲目资料修改历史 / 曲目数</dt>
                  <dd>{summary.metadataHistory ?? 0} / {summary.metadataHistorySongs ?? 0}</dd>
                </div>
                <div>
                  <dt>资料修改历史预计新增 / 重复 / 未定位 / 多路径待核对</dt>
                  <dd>{summary.metadataHistoryAdded ?? 0} / {summary.metadataHistoryDuplicates ?? 0} / {summary.metadataHistoryUnmatched ?? 0} / {summary.metadataHistoryAmbiguous ?? 0}</dd>
                </div>
                <div>
                  <dt>同编号内容冲突 · 计划 / 日期 / 方案</dt>
                  <dd>
                    {summary.conflicts} / {summary.dayConflicts} /{" "}
                    {summary.presetConflicts}
                  </dd>
                </div>
              </dl>
              <p className="parameter-help">
                累计时长按曲目和保存编号合并，相同条目只计算一次。旧版本备份没有累计时长条目时，现有累计时长保留。
              </p>
              {!!summary.timeLedgerConflicts && (
                <p role="alert">
                  本机与备份有 {summary.timeLedgerConflicts}{" "}
                  条同编号累计时长不同。系统会拒绝导入这部分成绩资料，以免重复计算。您可以取消“个人成绩与演奏回放”，先导入其他资料。
                </p>
              )}
              {!!summary.metadataHistoryError && selection.presets && (
                <p role="alert">本机资料修改历史无法安全合并：{summary.metadataHistoryError}。请先修复本机历史，或取消“片段、阶梯、技术练习、音轨、资料模板与修改历史”。</p>
              )}
              {!!summary.metadataHistoryUnmatched && selection.presets && (
                <p className="parameter-help">有 {summary.metadataHistoryUnmatched} 条资料修改历史没有找到已索引的同内容 MIDI。导入后，这些记录会保存在上方的待认领列表。</p>
              )}
              {!!summary.metadataHistoryAmbiguous && selection.presets && (
                <p className="parameter-help">有 {summary.metadataHistoryAmbiguous} 条资料修改历史对应本机多个相同内容路径。导入后，请在上方明确选择接收路径。</p>
              )}
              <p className="parameter-help">
                本机已有相同成绩 {summary.duplicateRecords} 条，合并时跳过。
              </p>
              <p className="parameter-help">
                回放随对应成绩一起迁移；旧备份没有回放时仍能导入成绩。已有相同回放{" "}
                {summary.duplicatePerformances ?? 0}{" "}
                次，合并时跳过。回放内容冲突按下方策略处理。
              </p>
              {!!summary.performanceErrors?.length && (
                <p role="alert">
                  本地有 {summary.performanceErrors.length}{" "}
                  次回放无法读取。选择“使用备份版本”可用已校验的备份恢复；保留本地时导入会中止，不会覆盖损坏资料。
                </p>
              )}
              <p>
                收藏的回放片段：{summary.replayClips ?? 0} 个。同编号片段冲突{" "}
                {summary.replayClipConflicts ?? 0}{" "}
                个，按所选策略保留本地或采用备份；双方各自新增的片段都会保留。
              </p>
              {!!summary.annotationConflicts && (
                <p className="parameter-help">
                  有 {summary.annotationConflicts}{" "}
                  条同次成绩的评语不同，按所选保留/采用备份策略处理；成绩不重复登记。
                </p>
              )}
              {!!summary.conflictRows.length && (
                <details>
                  <summary>查看资料与回放冲突</summary>
                  <ul>
                    {summary.conflictRows.map((r, i) => (
                      <li key={i}>
                        <span>
                          {r.kind} · 本地：{r.local}
                        </span>
                        <span>备份：{r.incoming}</span>
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              {summary.templateError && (
                <p role="alert">
                  本地模板读取失败：{summary.templateError}
                  。导入模板前请修复本地记录。
                </p>
              )}
              {!!summary.metadataTemplateRows?.length && (
                <details>
                  <summary>查看备份中的资料模板</summary>
                  {summary.metadataTemplateRows.map((template) => (
                    <div key={template.name}>
                      <h4>{template.name}</h4>
                      <table className="backup-template-table">
                        <thead>
                          <tr>
                            <th>字段</th>
                            <th>操作</th>
                            <th>内容</th>
                          </tr>
                        </thead>
                        <tbody>
                          {templateFields
                            .filter(
                              ([field]) =>
                                template.rules[field] &&
                                template.rules[field]!.mode !== "keep",
                            )
                            .map(([field, label]) => (
                              <tr key={field}>
                                <th>{label}</th>
                                <td>
                                  {
                                    modes(field).find(
                                      ([mode]) =>
                                        mode === template.rules[field]!.mode,
                                    )?.[1]
                                  }
                                </td>
                                <td>
                                  {template.rules[field]!.value || "（空）"}
                                </td>
                              </tr>
                            ))}
                        </tbody>
                      </table>
                    </div>
                  ))}
                </details>
              )}
              {!!summary.plans.length && (
                <details open>
                  <summary>计划与项目数量</summary>
                  <ul>
                    {summary.plans.map((p) => (
                      <li key={p.id}>
                        <span>
                          <span>{p.name}</span>
                          {p.notes && (
                            <small className="backup-plan-notes">
                              {p.notes}
                            </small>
                          )}
                        </span>
                        <span>
                          {p.items} 项
                          {p.schedule && (
                            <small className="backup-plan-notes">
                              {scheduleText(p.schedule)}
                            </small>
                          )}
                        </span>
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              {!!summary.references.length && (
                <details>
                  <summary>
                    计划与学习曲目定位 · {summary.references.length} 首
                  </summary>
                  <ul>
                    {summary.references.map((r) => (
                      <li key={r.contentId}>
                        <span>{r.title}</span>
                        <small>
                          {r.generated
                            ? "可重新生成"
                            : r.indexed
                              ? "已有索引，打开时核对内容"
                              : "尚无本机索引，打开时核对原位置；也可添加曲目包"}
                        </small>
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              <label className="backup-policy">
                同编号的计划、日期安排、曲目学习资料、方案与回放
                <select
                  aria-label="练习备份冲突策略"
                  disabled={busy}
                  value={policy}
                  onChange={(e) => setPolicy(e.target.value)}
                >
                  <option value="keep">保留本地版本，加入不存在的条目</option>
                  <option value="backup">使用备份版本，保留其他本地条目</option>
                </select>
              </label>
              <p className="parameter-help">
                成绩合并去重；同编号而内容不同的成绩保留为两条。每曲保留最近 200
                条，合并可能淘汰最早记录。历史计分条件原样保存，导入的计划改用本机延迟校准。导入前请暂停演奏并结束活动计划或阶梯。
              </p>
              <button
                className="primary-button"
                disabled={
                  busy ||
                  (selection.records && !!summary.timeLedgerConflicts) ||
                  (selection.presets && !!summary.metadataHistoryError) ||
                  (!available &&
                    !(
                      workspace &&
                      !workspace.saved &&
                      workspace.selected.length
                    ))
                }
                onClick={importFile}
              >
                {workspace
                  ? workspace.saved
                    ? "重试练习资料导入"
                    : "导入所选完整资料"
                  : "按所选范围导入"}
              </button>
            </>
          )}
        </div>
        {busy && !message && (
          <p role="status">{progress || "正在处理练习资料…"}</p>
        )}
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        {message && (
          <p className="backup-message" role="status">
            {message}
          </p>
        )}
      </section>
    </div>
  );
}
