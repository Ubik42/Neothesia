export interface ExpressionGoal {
  velocityDifference: number | null;
  noteVelocityDifference?: number | null;
  notePedalMismatchCount?: number | null;
  contourPercent: number | null;
  pedalOffsetMs: number | null;
}
export type NoteVelocityEvidence = {
  referenceNotes: number;
  matchedNotes: number;
  missingNotes: number;
  maxAbsDifference: number | null;
};
export type NotePedalEvidence = {
  referenceNotes: number;
  matchedNotes: number;
  pairedStateNotes: number;
  missingNotes: number;
  unknownStateNotes: number;
  mismatchCount: number;
};
export function notePedalEvidenceText(value?: NotePedalEvidence | null) {
  if (!value) return "";
  return `踏板状态可对照 ${value.pairedStateNotes} / ${value.referenceNotes} 音 · 不一致 ${value.mismatchCount} 音${value.unknownStateNotes ? ` · 状态未知 ${value.unknownStateNotes} 音` : ""}`;
}
export function noteVelocityEvidenceText(value?: NoteVelocityEvidence | null) {
  if (!value) return "";
  return `确切配对 ${value.matchedNotes} / ${value.referenceNotes} 音 · 最大力度差 ${value.maxAbsDifference ?? "无证据"}`;
}
export function expressionGoalText(goal?: ExpressionGoal | null) {
  if (!goal) return "";
  return [
    goal.velocityDifference != null
      ? `力度平均差 ≤ ${goal.velocityDifference}`
      : "",
    goal.noteVelocityDifference != null
      ? `逐音力度差 ≤ ${goal.noteVelocityDifference}`
      : "",
    goal.notePedalMismatchCount != null
      ? `逐音踏板不一致 ≤ ${goal.notePedalMismatchCount} 音`
      : "",
    goal.contourPercent != null ? `起伏一致率 ≥ ${goal.contourPercent}%` : "",
    goal.pedalOffsetMs != null ? `踏板偏移 ≤ ${goal.pedalOffsetMs} ms` : "",
  ]
    .filter(Boolean)
    .join(" · ");
}
export function ExpressionGoalControl({
  value,
  disabled,
  change,
  wait = false,
  labelPrefix = "计划项目",
}: {
  value?: ExpressionGoal | null;
  disabled: boolean;
  wait?: boolean;
  labelPrefix?: string;
  change: (g: ExpressionGoal | null) => void;
}) {
  const update = (key: keyof ExpressionGoal, v: number | null) => {
    const next = {
      velocityDifference: null,
      noteVelocityDifference: null,
      notePedalMismatchCount: null,
      contourPercent: null,
      pedalOffsetMs: null,
      ...value,
      [key]: v,
    };
    change(Object.values(next).some((v) => v != null) ? next : null);
  };
  const rows = [
    {
      key: "velocityDifference" as const,
      label: "力度平均差",
      hint: "至少 4 个匹配音符",
      initial: 12,
      min: 1,
      max: 64,
      unit: " / 127",
    },
    {
      key: "noteVelocityDifference" as const,
      label: "逐音力度差",
      hint: "至少 4 个确切配对音；漏音或无逐音记录不能达标",
      initial: 20,
      min: 1,
      max: 64,
      unit: " / 127",
    },
    {
      key: "contourPercent" as const,
      label: "力度起伏一致率",
      hint: "至少 6 段参考变化",
      initial: 80,
      min: 50,
      max: 100,
      unit: "%",
    },
    {
      key: "pedalOffsetMs" as const,
      label: "踏板时机偏移",
      hint: "至少 4 次踏板转换且全部对应",
      initial: 80,
      min: 10,
      max: 500,
      unit: " ms",
    },
    {
      key: "notePedalMismatchCount" as const,
      label: "逐音踏板不一致音数",
      hint: "参考与实际起音前都需有明确踏板状态；至少 4 个确切配对音，漏音或状态未知不能达标",
      initial: 0,
      min: 0,
      max: 8,
      unit: " 音",
    },
  ];
  return (
    <fieldset className="expression-goal-control" disabled={disabled}>
      <legend>力度与踏板要求（可选）</legend>
      {rows.map((row) => (
        <div key={row.key}>
          <label>
            <input
              type="checkbox"
              disabled={
                wait &&
                row.key === "pedalOffsetMs" &&
                value?.pedalOffsetMs == null
              }
                aria-label={`${labelPrefix}要求${row.label}`}
              checked={value?.[row.key] != null}
              onChange={(e) =>
                update(row.key, e.target.checked ? row.initial : null)
              }
            />
            {row.label}
          </label>
          {value?.[row.key] != null && (
            <label className="expression-threshold">
              {row.key === "contourPercent" ? "至少" : "最多"}
              <input
                type="number"
                aria-label={`${labelPrefix}${row.label}`}
                min={row.min}
                max={row.max}
                value={value[row.key]!}
                onChange={(e) => update(row.key, Number(e.target.value))}
              />
              {row.unit}
            </label>
          )}
          <small>{row.hint}</small>
        </div>
      ))}
      <p className="parameter-help">
        对照参考
        MIDI；键盘力度曲线会影响绝对差值，也可只选起伏方向。逐音力度和踏板状态会逐一核对计分器确切配对音。踏板状态缺少明确控制事件时不会猜为松开。踏板时机不适用于等音。参考范围或本轮数据不足时不会达标；踏板波动还需不超过
        120 ms。
      </p>
    </fieldset>
  );
}
