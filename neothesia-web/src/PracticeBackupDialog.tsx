import { templateFields, modes, type Rules } from "./metadataTemplates";
import { useRef, useState, useEffect } from "react";
import { X, Download, FolderOpen } from "lucide-react";
import { api } from "./api";
import { scheduleText, type RoutineSchedule } from "./RoutineScheduleEditor";
type Summary = {
  replayClips?: number;
  replayClipConflicts?: number;
  performances?: number;
  performanceBytes?: number;
  performanceConflicts?: number;
  duplicatePerformances?: number;
  performanceErrors?: { contentId: string; id: string; error: string }[];
  metadataTemplates?: number;
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
  const input = useRef<HTMLInputElement>(null),
    working = useRef(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const work = async (job: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    setMessage("");
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
  const importFile = () =>
    void work(async () => {
      if (!summary) return;
      const result = await api.command<{
        importedPerformances: number;
        replacedPerformances: number;
        keptPerformances: number;
        duplicatePerformances: number;
        trimmedPerformances: number;
        addedRecords: number;
        duplicateRecords: number;
        renamedRecords: number;
        trimmedRecords: number;
      }>({
        type: "importStagedPracticeBackup",
        token: summary.token,
        selection: { ...selection, days: 0 },
        policy,
      });
      await changed();
      setMessage(
        `导入完成：新增记录 ${result.addedRecords}，跳过重复 ${result.duplicateRecords}，区分同编号记录 ${result.renamedRecords}${result.trimmedRecords ? `；每曲保留上限淘汰最早 ${result.trimmedRecords} 条` : ""}。回放新增 ${result.importedPerformances ?? 0} 次、恢复/替换 ${result.replacedPerformances ?? 0} 次、保留本地 ${result.keptPerformances ?? 0} 次、跳过重复 ${result.duplicatePerformances ?? 0} 次${result.trimmedPerformances ? `，随淘汰成绩跳过 ${result.trimmedPerformances} 次` : ""}。本机导入前副本已保留。`,
      );
      setSummary(null);
    });
  const any = selection.plans || selection.records || selection.presets;
  const available =
    summary &&
    ((selection.plans &&
      (summary.plans.length || summary.days || summary.learning)) ||
      (selection.records && summary.records) ||
      (selection.presets &&
        summary.passages +
          summary.ladders +
          summary.exercises +
          (summary.trackSounds ?? 0) +
          (summary.trackAppearances ?? 0) +
          (summary.metadataTemplates ?? 0)));
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
          备份按曲目内容身份关联。MIDI、谱面、指法、逐音分手及拍号修正请通过曲目包迁移；这里保留练习计划、日期安排、曲目学习资料、片段、阶梯、资料模板、成绩历史和已有演奏回放。设备选择与设备校准保留本机设置。
        </p>
        <div className="practice-backup-selection">
          {[
            ["plans", "计划、日课与曲目学习"],
            ["records", "个人成绩与演奏回放"],
            ["presets", "片段、阶梯、技术练习、音轨与资料模板"],
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
        <div className="practice-backup-export">
          <h3>保存本机资料</h3>
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
                  setSummary((await api.stagePracticeBackup(file)) as Summary);
                });
            }}
          />
          {summary && (
            <>
              <p>
                备份日期：{new Date(summary.createdAt).toLocaleString("zh-CN")}
              </p>
              <dl className="practice-backup-counts">
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
                  <dt>同编号内容冲突 · 计划 / 日期 / 方案</dt>
                  <dd>
                    {summary.conflicts} / {summary.dayConflicts} /{" "}
                    {summary.presetConflicts}
                  </dd>
                </div>
              </dl>
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
                disabled={busy || !available}
                onClick={importFile}
              >
                按所选范围导入
              </button>
            </>
          )}
        </div>
        {busy && <p role="status">正在处理练习资料…</p>}
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
