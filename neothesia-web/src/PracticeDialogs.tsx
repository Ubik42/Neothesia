import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { api, type Feedback, type Snapshot } from "./api";
import { noteVelocityEvidenceText, notePedalEvidenceText } from "./ExpressionGoalControl";
interface ExerciseSpec {
  tonic: number;
  tonality: string;
  minor_form: string;
  pattern: string;
  direction: string;
  hands: string;
  octaves: number;
  repetitions: number;
  tempo_bpm: number;
}
export function ExerciseDialog({
  close,
  generate,
}: {
  close: () => void;
  generate: (spec: ExerciseSpec) => Promise<void>;
}) {
  const [presets, setPresets] = useState<
      { id: string; name: string; spec: ExerciseSpec }[]
    >([]),
    [presetName, setPresetName] = useState(""),
    [presetError, setPresetError] = useState("");
  const loadPresets = async () =>
    setPresets(
      (
        await api.command<{ presets: typeof presets }>({
          type: "exercisePresets",
        })
      ).presets,
    );
  useEffect(() => {
    void loadPresets().catch((e) => setPresetError(String(e.message)));
  }, []);
  const [spec, setSpec] = useState<ExerciseSpec>({
      tonic: 0,
      tonality: "Major",
      minor_form: "Natural",
      pattern: "Scale",
      direction: "UpAndDown",
      hands: "Both",
      octaves: 1,
      repetitions: 1,
      tempo_bpm: 60,
    }),
    [busy, setBusy] = useState(false);
  const field = (
    key: keyof ExerciseSpec,
    label: string,
    options: [string | number, string][],
  ) => (
    <label key={key}>
      {label}
      <select
        aria-label={label}
        value={spec[key]}
        onChange={(e) =>
          setSpec({
            ...spec,
            [key]:
              typeof spec[key] === "number"
                ? Number(e.target.value)
                : e.target.value,
          })
        }
      >
        {options.map(([value, title]) => (
          <option key={value} value={value}>
            {title}
          </option>
        ))}
      </select>
    </label>
  );
  return (
    <div className="modal-backdrop" onClick={close}>
      <section
        className="settings-dialog exercise-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="exercise-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <h2 id="exercise-title">技术练习</h2>
          <button autoFocus aria-label="关闭技术练习" onClick={close}>
            <X size={18} />
          </button>
        </div>
        <div className="exercise-presets">
          <label>
            常用方案
            <select
              aria-label="常用技术练习方案"
              value=""
              onChange={(e) => {
                const p = presets.find((p) => p.id === e.target.value);
                if (p) {
                  setSpec(p.spec);
                  setPresetName(p.name);
                }
              }}
            >
              <option value="">选择已保存的方案</option>
              {presets.map((p) => (
                <option value={p.id} key={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            方案名称
            <input
              aria-label="技术练习方案名称"
              placeholder="例如：每天两八度音阶"
              value={presetName}
              onChange={(e) => setPresetName(e.target.value)}
            />
          </label>
          <button
            disabled={busy || !presetName.trim()}
            onClick={async () => {
              setBusy(true);
              setPresetError("");
              try {
                await api.command({
                  type: "saveExercisePreset",
                  name: presetName,
                  spec,
                });
                await loadPresets();
              } catch (e) {
                setPresetError(e instanceof Error ? e.message : String(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            保存方案
          </button>
        </div>
        {presetError && (
          <p role="alert" className="dialog-error">
            {presetError}
          </p>
        )}
        {presets.length > 0 && (
          <details className="preset-management">
            <summary>管理已保存方案</summary>
            {presets.map((p) => (
              <div key={p.id}>
                <span>{p.name}</span>
                <button
                  onClick={() => {
                    setSpec(p.spec);
                    setPresetName(p.name);
                  }}
                >
                  使用
                </button>
                <button
                  disabled={busy}
                  onClick={async () => {
                    try {
                      await api.command({
                        type: "removeExercisePreset",
                        id: p.id,
                      });
                      await loadPresets();
                    } catch (e) {
                      setPresetError(String(e));
                    }
                  }}
                >
                  移除
                </button>
              </div>
            ))}
          </details>
        )}
        <div className="exercise-grid">
          {field(
            "tonic",
            "主音",
            [
              "C",
              "C♯",
              "D",
              "E♭",
              "E",
              "F",
              "F♯",
              "G",
              "A♭",
              "A",
              "B♭",
              "B",
            ].map((s, i) => [i, s]),
          )}
          {field("tonality", "调式", [
            ["Major", "大调"],
            ["Minor", "小调"],
          ])}
          {spec.tonality === "Minor" &&
            field("minor_form", "小调形式", [
              ["Natural", "自然小调"],
              ["Harmonic", "和声小调"],
              ["Melodic", "旋律小调"],
            ])}
          {field("pattern", "练习类型", [
            ["Scale", "音阶"],
            ["Arpeggio", "琶音"],
            ["PrimaryChords", "主三和弦"],
          ])}
          {field("direction", "练习方向", [
            ["Ascending", "上行"],
            ["Descending", "下行"],
            ["UpAndDown", "上行与下行"],
          ])}
          {field("hands", "生成练习声部", [
            ["Both", "双手"],
            ["Right", "右手"],
            ["Left", "左手"],
          ])}
          {field(
            "octaves",
            "练习八度",
            [1, 2, 3, 4].map((n) => [n, String(n)]),
          )}
          <label>
            重复次数
            <input
              aria-label="练习重复次数"
              type="number"
              min="1"
              max="8"
              value={spec.repetitions}
              onChange={(e) =>
                setSpec({ ...spec, repetitions: Number(e.target.value) })
              }
            />
          </label>
          <label>
            速度（BPM）
            <input
              aria-label="生成练习速度"
              type="number"
              min="20"
              max="240"
              value={spec.tempo_bpm}
              onChange={(e) =>
                setSpec({ ...spec, tempo_bpm: Number(e.target.value) })
              }
            />
          </label>
        </div>
        <p className="parameter-help">
          生成后直接进入练习。经校对的音阶和琶音指法会显示在音符上；其余组合不提供未经校对的指法。
        </p>
        <button
          className="primary"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await generate(spec);
            } finally {
              setBusy(false);
            }
          }}
        >
          {busy ? "正在生成…" : "生成并打开"}
        </button>
      </section>
    </div>
  );
}
const percentage = (b: {
  matched_notes: number;
  wrong_notes: number;
  missed_notes: number;
}) => {
  const total = b.matched_notes + b.wrong_notes + b.missed_notes;
  return total ? `${Math.round((b.matched_notes / total) * 100)}%` : "—";
};
export type FeedbackPlanSource = {
  contentId: string;
  id: string;
  title: string;
  recordedAt: number;
  total: number;
  notes: { reference: number; measure: number; pitch: number; kind: string; velocityDifference?: number; velocityLimit?: number; referenceVelocity?: number; actualVelocity?: number; timingKind?: string | null }[];
};
export type MeasurePartFeedback = NonNullable<Feedback["measure_parts"]>[number];
const feedbackPartName = (part: MeasurePartFeedback["part"]) =>
  part === "LeftHand" ? "左手" : part === "RightHand" ? "右手" : "其他声部";
export function FeedbackDialog({
  close,
  state,
  contentId,
  practice,
  practicePart,
  trackNames,
  retry,
  arrangeNext,
}: {
  close: () => void;
  state: Snapshot;
  contentId: string | null;
  practice: (start: number, end: number) => Promise<void>;
  practicePart: (target: MeasurePartFeedback) => Promise<void>;
  trackNames: Record<number, string>;
  retry: () => Promise<void>;
  arrangeNext: (source: FeedbackPlanSource) => void;
}) {
  const [summary, setSummary] = useState<Feedback | null>(state.summary),
    [error, setError] = useState(""),
    [planSource, setPlanSource] = useState<FeedbackPlanSource | null>(null),
    [planLoading, setPlanLoading] = useState(false),
    [selectedPart, setSelectedPart] = useState<MeasurePartFeedback | null>(null),
    [partLoading, setPartLoading] = useState(false);
  useEffect(() => {
    api
      .command<Feedback>({ type: "feedback" })
      .then(setSummary)
      .catch((e) => setError(String(e.message)));
  }, []);
  useEffect(() => {
    const id = state.currentHistoryId;
    if (!state.saved || !contentId || !id || state.routine || state.ladder?.active) {
      setPlanSource(null);
      return;
    }
    let cancelled = false;
    setPlanLoading(true);
    void api.command<FeedbackPlanSource>({
      type: "historyProblemNotes",
      content_id: contentId,
      id,
      offset: 0,
    }).then((source) => {
      if (!cancelled && source.contentId === contentId && source.id === id)
        setPlanSource(source.total > 0 && source.notes.length > 0 ? source : null);
    }).catch(() => {
      if (!cancelled) setPlanSource(null);
    }).finally(() => {
      if (!cancelled) setPlanLoading(false);
    });
    return () => { cancelled = true; };
  }, [contentId, state.saved, state.currentHistoryId, state.routine, state.ladder?.active]);
  const weakest = summary?.measures
    .filter((m) => m.breakdown.missed_notes + m.breakdown.wrong_notes > 0)
    .sort(
      (a, b) =>
        b.breakdown.missed_notes +
        b.breakdown.wrong_notes -
        (a.breakdown.missed_notes + a.breakdown.wrong_notes),
    )[0];
  const measureParts = summary?.measure_parts ?? [];
  const problemParts = measureParts.filter(
    ({ breakdown: b }) => b.missed_notes + b.early_notes + b.late_notes > 0,
  );
  const selectedPartLabel = selectedPart
    ? selectedPart.part === "Other"
      ? `第 ${selectedPart.measure} 小节 · 其他声部 · ${trackNames[selectedPart.track_id] ?? `音轨 ${selectedPart.track_id + 1}`}`
      : `第 ${selectedPart.measure} 小节 · ${feedbackPartName(selectedPart.part)} · ${trackNames[selectedPart.track_id] ?? `音轨 ${selectedPart.track_id + 1}`}`
    : "";
  return (
    <div className="modal-backdrop" onClick={close}>
      <section
        className="settings-dialog feedback-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="feedback-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <h2 id="feedback-title">
            {["recital", "memory"].includes(summary?.mode ?? state.mode)
              ? "演奏完成"
              : "逐小节反馈"}
          </h2>
          <button autoFocus aria-label="关闭练习反馈" onClick={close}>
            <X size={18} />
          </button>
        </div>
        {error && <p role="alert">{error}</p>}
        {summary && (
          <>
            {["recital", "memory"].includes(summary.mode ?? state.mode) && (
              <button onClick={() => void retry()}>再演奏一遍</button>
            )}
            <div className="feedback-overview">
              <strong>{percentage(summary.overall)}</strong>
              <span>
                正确 {summary.overall.matched_notes} · 错音{" "}
                {summary.overall.wrong_notes} · 漏音{" "}
                {summary.overall.missed_notes}
              </span>
            </div>
            {state.songExpressionGoalResult && !state.routine && !state.ladder?.active && (
              <p className={state.songExpressionGoalResult?.passed ? "song-expression-passed" : "song-expression-failed"}>
                本轮目标：{state.songExpressionGoalResult.message}
                {state.songExpressionGoalResult.noteVelocity && <> · {noteVelocityEvidenceText(state.songExpressionGoalResult.noteVelocity)}</>}
                {state.songExpressionGoalResult.notePedal && <> · {notePedalEvidenceText(state.songExpressionGoalResult.notePedal)}</>}
              </p>
            )}
            {["flow", "recital", "memory"].includes(
              summary.mode ?? state.mode,
            ) ? (
              <p>
                节奏偏移中位数 {summary.timing.median_offset_ms ?? "—"} ms ·
                波动 {summary.timing.median_deviation_ms ?? "—"} ms
                <br />
                完整和弦 {summary.chords.complete_chords} /{" "}
                {summary.chords.eligible_chords} · 起音跨度{" "}
                {summary.chords.median_attack_span_ms ?? "—"} ms
              </p>
            ) : (
              <p className="parameter-help">
                等音模式不评价节奏。请使用连续模式练习节奏、和弦起音与音符时值。
              </p>
            )}
            {["flow", "recital", "memory"].includes(
              summary.mode ?? state.mode,
            ) && (
              <p className="parameter-help">
                力度：
                {summary.expression.velocity.matched_samples >= 4
                  ? `平均差 ${summary.expression.velocity.mean_abs_difference ?? "—"} / 127`
                  : "样本不足"}{" "}
                · 力度走向：
                {summary.expression.velocity.contour_steps >= 6
                  ? `${summary.expression.velocity.contour_aligned}/${summary.expression.velocity.contour_steps} 段一致`
                  : "样本不足"}
                <br />
                踏板：
                {summary.expression.pedal.target_present
                  ? `有效对照 ${summary.expression.pedal.timing_samples} 次，偏移 ${summary.expression.pedal.median_offset_ms ?? "—"} ms`
                  : "参考 MIDI 未提供踏板，暂不评判"}{" "}
                · 音符时值：
                {summary.expression.articulation.matched_samples >= 4
                  ? `${summary.expression.articulation.median_duration_ratio_percent ?? "—"}%`
                  : "样本不足"}
              </p>
            )}
            {summary.trend && (
              <p className="parameter-help">
                同段落、声部、模式共 {summary.trend.attempts} 次记录
                {summary.trend.accuracyDelta !== null
                  ? ` · 准确率变化 ${Math.round(summary.trend.accuracyDelta * 100)} 个百分点`
                  : ""}
                {summary.trend.tempoDelta !== null
                  ? ` · 速度变化 ${summary.trend.tempoDelta} BPM`
                  : ""}
              </p>
            )}
            <div className="feedback-parts">
              {summary.parts.map((p) => (
                <span key={p.part}>
                  {p.part === "LeftHand"
                    ? "左手"
                    : p.part === "RightHand"
                      ? "右手"
                      : "其他声部"}{" "}
                  {percentage(p.breakdown)}
                </span>
              ))}
            </div>
            <section className="feedback-part-review" aria-label="本轮分手问题">
              <h3>本轮小节与声部</h3>
              {measureParts.length === 0 ? (
                <p className="parameter-help">本轮没有可核对的小节与声部记录。旧记录不会按曲目当前分手重新推断。</p>
              ) : problemParts.length === 0 ? (
                <p className="parameter-help">本轮有 {measureParts.length} 组已评判的目标音，没有偏早、偏晚或漏音。</p>
              ) : (
                <>
                  <p className="parameter-help">下列问题来自本轮已评判的目标音。无法定位到目标音的误弹只计入总成绩，不会猜测归属到某只手。</p>
                  <div className="feedback-table feedback-part-table">
                    <table>
                      <thead><tr><th>小节</th><th>声部 / 音轨</th><th>目标音</th><th>偏早</th><th>偏晚</th><th>漏音</th><th /></tr></thead>
                      <tbody>
                        {problemParts.map((item) => (
                          <tr key={`${item.measure}:${item.part}:${item.track_id}`}>
                            <td>{item.measure}</td>
                            <td>{feedbackPartName(item.part)} · {trackNames[item.track_id] ?? `音轨 ${item.track_id + 1}`}</td>
                            <td>{item.breakdown.matched_notes}/{item.breakdown.target_notes}</td>
                            <td>{item.breakdown.early_notes}</td>
                            <td>{item.breakdown.late_notes}</td>
                            <td>{item.breakdown.missed_notes}</td>
                            <td><button onClick={() => setSelectedPart(item)}>准备重练</button></td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}
              {selectedPart && (
                <div className="feedback-part-prepare" role="group" aria-label="分手重练设置预览">
                  <strong>重练设置：{selectedPartLabel}</strong>
                  <p>重练会只将这个音轨中所选声部的目标音交给你弹，其他音轨或声部作为伴奏。循环范围为第 {selectedPart.measure} 小节。{state.routine || state.ladder?.active ? "当前计划或速度阶梯会先结束。" : ""}{["recital", "memory", "listen"].includes(state.mode) ? "当前模式会先切换到连续练习。" : ""}{state.recording ? "请先停止录音。" : ""}</p>
                  <div>
                    <button disabled={partLoading} onClick={() => setSelectedPart(null)}>取消</button>
                    <button className="primary" disabled={partLoading || state.recording} onClick={() => {
                      setPartLoading(true);
                      void practicePart(selectedPart).catch((e) => setError(String(e instanceof Error ? e.message : e))).finally(() => setPartLoading(false));
                    }}>应用设置并重练</button>
                  </div>
                </div>
              )}
            </section>
            {weakest && (
              <button
                onClick={() => void practice(weakest.measure, weakest.measure)}
              >
                重练第 {weakest.measure} 小节
              </button>
            )}
            {planSource && state.saved && state.currentHistoryId === planSource.id && (
              <div className="feedback-next-practice">
                <p>本轮保存的演奏记录发现 {planSource.total} 个偏早、偏晚、漏音或逐音力度超限。这是单次练习证据，不代表长期弱项。</p>
                <button className="primary" disabled={planLoading} onClick={() => arrangeNext(planSource)}>
                  预览并安排下次练习
                </button>
              </div>
            )}
            <div className="feedback-table">
              <table>
                <thead>
                  <tr>
                    <th>小节</th>
                    <th>准确率</th>
                    <th>错音</th>
                    <th>漏音</th>
                    <th>节奏偏移</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {summary.measures.map((m) => (
                    <tr key={m.measure}>
                      <td>{m.measure}</td>
                      <td>{percentage(m.breakdown)}</td>
                      <td>{m.breakdown.wrong_notes}</td>
                      <td>{m.breakdown.missed_notes}</td>
                      <td>
                        {["flow", "recital", "memory"].includes(
                          summary.mode ?? state.mode,
                        )
                          ? `${m.timing.median_offset_ms ?? "—"} ms`
                          : "—"}
                      </td>
                      <td>
                        <button
                          onClick={() => void practice(m.measure, m.measure)}
                        >
                          重练
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        )}
      </section>
    </div>
  );
}
interface Metadata {
  title: string | null;
  composer: string | null;
  artist: string | null;
  collection: string | null;
  difficulty: string | null;
  tags: string[];
  notes: string | null;
}
export function MetadataDialog({
  close,
  title,
  save,
}: {
  close: () => void;
  title: string;
  save: (value: Metadata) => Promise<void>;
}) {
  const [value, setValue] = useState<Metadata>({
      title,
      composer: null,
      artist: null,
      collection: null,
      difficulty: null,
      tags: [],
      notes: null,
    }),
    [busy, setBusy] = useState(false);
  useEffect(() => {
    api
      .command<Metadata>({ type: "metadata" })
      .then((data) => setValue({ ...data, title: data.title || title }));
  }, [title]);
  return (
    <div className="modal-backdrop" onClick={close}>
      <section
        className="settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="metadata-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <h2 id="metadata-title">曲目信息</h2>
          <button autoFocus aria-label="关闭曲目信息" onClick={close}>
            <X size={18} />
          </button>
        </div>
        {(
          [
            ["title", "曲名"],
            ["composer", "作曲家"],
            ["artist", "演奏者"],
            ["collection", "作品集"],
            ["difficulty", "难度"],
            ["notes", "备注"],
          ] as const
        ).map(([key, label]) => (
          <label key={key}>
            {label}
            <input
              value={value[key] || ""}
              onChange={(e) =>
                setValue({ ...value, [key]: e.target.value || null })
              }
            />
          </label>
        ))}
        <label>
          标签（逗号分隔）
          <input
            value={value.tags.join(",")}
            onChange={(e) =>
              setValue({
                ...value,
                tags: e.target.value
                  .split(/[,，]/)
                  .map((s) => s.trim())
                  .filter(Boolean),
              })
            }
          />
        </label>
        <button
          className="primary"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await save(value);
            } finally {
              setBusy(false);
            }
          }}
        >
          保存
        </button>
      </section>
    </div>
  );
}
