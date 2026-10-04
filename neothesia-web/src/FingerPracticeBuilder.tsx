import { useState } from "react";
import { api, pitchName } from "./api";

export type FingerDifficulty = {
  measure: number;
  fromMeasure: number;
  pitch: number;
  fromPitch: number;
  kind:
    | "wideReach"
    | "rapidTurn"
    | "rapidShift"
    | "sameFingerMove"
    | "sameKeyHandoff"
    | "crossedRange";
  instruction?: string;
  intervalMs: number | null;
  semitones: number;
  comfortableSemitones: number;
};
export type Draft = {
  selected: boolean;
  name: string;
  start: number;
  end: number;
  speed: number;
  rounds: number;
  notes: string;
};
const labels = {
  sameKeyHandoff: "同键交接",
  crossedRange: "音域交叠",
  wideReach: "跨度",
  rapidTurn: "穿 / 跨指",
  rapidShift: "移手",
  sameFingerMove: "同指换键",
};

function drafts(
  review: FingerDifficulty[],
  rate: number,
  hand: string,
): Draft[] {
  const groups: { start: number; end: number; issues: FingerDifficulty[] }[] =
    [];
  for (const issue of [...review].sort(
    (a, b) =>
      Math.min(a.measure, a.fromMeasure) - Math.min(b.measure, b.fromMeasure),
  )) {
    const start = Math.min(issue.measure, issue.fromMeasure),
      end = Math.max(issue.measure, issue.fromMeasure);
    const last = groups.at(-1);
    if (last && start <= last.end) {
      last.end = Math.max(last.end, end);
      last.issues.push(issue);
    } else groups.push({ start, end, issues: [issue] });
  }
  return groups.slice(0, 32).map((g) => {
    const speed = g.issues.reduce(
      (speed, r) =>
        r.kind === "sameKeyHandoff" || r.kind === "crossedRange"
          ? Math.min(speed, rate, 0.5)
          : r.kind !== "wideReach" && r.intervalMs !== null
            ? Math.min(
                speed,
                (rate * r.intervalMs) / (r.kind === "rapidTurn" ? 200 : 280),
              )
            : speed,
      rate,
    );
    const kinds = [...new Set(g.issues.map((r) => labels[r.kind]))].join("、");
    return {
      selected: true,
      name: `${hand === "both" ? "双手" : hand === "left" ? "左手" : "右手"} · ${kinds} · ${g.start === g.end ? g.start : `${g.start}–${g.end}`} 小节`,
      start: g.start,
      end: g.end,
      speed: Math.max(0.25, Math.floor(speed * 100) / 100),
      rounds: 3,
      notes: g.issues
        .map((r) =>
          r.instruction
            ? `第 ${r.fromMeasure}→${r.measure} 小节：${r.instruction}`
            : `第 ${r.fromMeasure}→${r.measure} 小节 ${pitchName(r.fromPitch)}→${pitchName(r.pitch)}：${labels[r.kind]}${r.kind === "wideReach" ? ` ${r.semitones} 半音（当前手型 ${r.comfortableSemitones} 半音），检查持音、分手与手位` : ` ${r.intervalMs} 毫秒，先慢练衔接再逐步提速`}`,
        )
        .join("\n"),
    };
  });
}

export function FingerPracticeBuilder({
  review,
  rate,
  hand,
  request,
  fingerprint,
  measures,
  disabled,
  changed,
  pair,
  fallback,
  busyChanged,
}: {
  review: FingerDifficulty[];
  rate: number;
  hand: string;
  request: Record<string, unknown>;
  fingerprint: string;
  measures: number;
  disabled: boolean;
  changed: () => Promise<void>;
  pair?: { parts: unknown[]; proof: string; acknowledged: boolean };
  fallback?: Draft[];
  busyChanged?: (busy: boolean) => void;
}) {
  const [open, setOpen] = useState(false),
    [rows, setRows] = useState<Draft[]>([]),
    [busy, setBusy] = useState(false),
    [message, setMessage] = useState("");
  const update = (i: number, patch: Partial<Draft>) =>
    setRows(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const selected = rows.filter((r) => r.selected);
  const valid =
    selected.length > 0 &&
    selected.every(
      (r) =>
        r.name.trim().length > 0 &&
        r.name.length <= 80 &&
        Number.isInteger(r.start) &&
        Number.isInteger(r.end) &&
        r.start >= 1 &&
        r.end >= r.start &&
        r.end <= measures &&
        Number.isFinite(r.speed) &&
        r.speed >= 0.25 &&
        r.speed <= 2 &&
        Number.isInteger(r.rounds) &&
        r.rounds >= 1 &&
        r.rounds <= 99,
    );
  async function save() {
    busyChanged?.(true);
    setBusy(true);
    setMessage("");
    try {
      const result = await api.command<{ saved: number; existing?: number }>(
        pair
          ? {
              type: "savePairFingerPractice",
              ...pair,
              ranges: selected.map(({ selected: _, ...r }) => r),
            }
          : {
              type: "saveFingerPractice",
              request,
              fingerprint,
              ranges: selected.map(({ selected: _, ...r }) => r),
            },
      );
      setMessage(
        `已保存 ${result.saved} 个练习段${result.existing ? `，${result.existing} 个相同练习段已存在` : ""}，可在练习段列表打开，或加入日课。指法仍需在本窗口确认保存。`,
      );
      setOpen(false);
      await changed();
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
      busyChanged?.(false);
    }
  }
  return (
    <section className="finger-practice-builder" aria-label="指法难点练习段">
      <button
        disabled={disabled || busy || (!review.length && !fallback?.length)}
        onClick={() => {
          setRows(
            review.length ? drafts(review, rate, hand) : (fallback ?? []),
          );
          setMessage("");
          setOpen(!open);
        }}
      >
        {hand === "both"
          ? review.length
            ? "整理双手难点练习段"
            : "建立双手练习段"
          : "整理为练习段"}
      </button>
      {open && (
        <>
          <p>
            重叠的小节合为一段，最多一次保存 32
            段。速度按当前复核间隔估算，是慢练起点。跨度问题需调整手位或分手。练习使用已有指法，本操作不会保存尚未接受的建议。
          </p>
          {rows.map((r, i) => (
            <fieldset key={i} disabled={busy || disabled}>
              <legend>
                <label>
                  <input
                    type="checkbox"
                    aria-label={`保存难点练习段 ${i + 1}`}
                    checked={r.selected}
                    onChange={(e) => update(i, { selected: e.target.checked })}
                  />
                  练习段 {i + 1}
                </label>
              </legend>
              <label>
                名称
                <input
                  aria-label={`难点段名称 ${i + 1}`}
                  value={r.name}
                  maxLength={80}
                  onChange={(e) => update(i, { name: e.target.value })}
                />
              </label>
              <div className="finger-practice-fields">
                <label>
                  起始小节
                  <input
                    aria-label={`难点段起始小节 ${i + 1}`}
                    type="number"
                    min={1}
                    max={measures}
                    value={r.start}
                    onChange={(e) =>
                      update(i, { start: Number(e.target.value) })
                    }
                  />
                </label>
                <label>
                  结束小节
                  <input
                    aria-label={`难点段结束小节 ${i + 1}`}
                    type="number"
                    min={r.start}
                    max={measures}
                    value={r.end}
                    onChange={(e) => update(i, { end: Number(e.target.value) })}
                  />
                </label>
                <label>
                  原速 %
                  <input
                    aria-label={`难点段速度 ${i + 1}`}
                    type="number"
                    min={25}
                    max={200}
                    value={Math.round(r.speed * 100)}
                    onChange={(e) =>
                      update(i, { speed: Number(e.target.value) / 100 })
                    }
                  />
                </label>
                <label>
                  重复次数
                  <input
                    aria-label={`难点段重复次数 ${i + 1}`}
                    type="number"
                    min={1}
                    max={99}
                    value={r.rounds}
                    onChange={(e) =>
                      update(i, { rounds: Number(e.target.value) })
                    }
                  />
                </label>
              </div>
              <label>
                练习备注
                <textarea
                  aria-label={`难点段备注 ${i + 1}`}
                  value={r.notes}
                  onChange={(e) => update(i, { notes: e.target.value })}
                />
              </label>
            </fieldset>
          ))}
          <div className="finger-practice-actions">
            <button
              disabled={disabled || busy || !valid}
              onClick={() => void save()}
            >
              {busy
                ? "正在保存…"
                : `保存 ${selected.length} 个${hand === "both" ? "双手" : "难点"}练习段`}
            </button>
            <button disabled={busy} onClick={() => setOpen(false)}>
              取消整理
            </button>
          </div>
        </>
      )}
      {message && <p role="status">{message}</p>}
    </section>
  );
}
