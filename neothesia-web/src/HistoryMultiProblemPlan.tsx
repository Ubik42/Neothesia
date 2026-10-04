import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { HistoryProblemPlan } from "./HistoryProblemPlan";
import type { HistoryScoreReview } from "./historyScoreFocus";
export type ReviewSource = {
  contentId: string;
  id: string;
  title: string;
  recordedAt: number;
  slot: number;
};
type Notes = {
  reference: number;
  measure: number;
  pitch: number;
  kind: string;
};
export function HistoryMultiProblemPlan({
  sources,
  remove,
  openPlan,
  dirtyChanged,
}: {
  sources: ReviewSource[];
  remove: (id: string, contentId: string) => void;
  openPlan: (id: string, day: string | null) => void;
  dirtyChanged: (dirty: boolean) => void;
}) {
  const [loaded, setLoaded] = useState<
      Record<string, { notes: Notes[]; total: number; error: string }>
    >({}),
    [busy, setBusy] = useState(false);
  const epoch = useRef(0);
  useEffect(() => {
    const version = ++epoch.current;
    let alive = true;
    setBusy(true);
    void Promise.all(
      sources.map(async (s) => {
        try {
          const data = await api.command<{ notes: Notes[]; total: number }>({
            type: "historyProblemNotes",
            content_id: s.contentId,
            id: s.id,
            offset: 0,
          });
          return [s.slot, { ...data, error: "" }] as const;
        } catch (e) {
          return [s.slot, { notes: [], total: 0, error: String(e) }] as const;
        }
      }),
    ).then((rows) => {
      if (alive) {
        setLoaded(Object.fromEntries(rows));
        setBusy(false);
      }
    });
    return () => {
      alive = false;
      if (epoch.current === version) epoch.current++;
    };
  }, [sources]);
  const review: HistoryScoreReview = {
    contentId: "multi",
    historyId: "multi",
    scoreRevision: 0,
    recordedAt: Math.max(0, ...sources.map((s) => s.recordedAt)),
    title: `${sources.length} 条演奏记录`,
    exact: true,
    truncated: false,
    wrongNotes: null,
    notes: sources.flatMap((s) =>
      (loaded[s.slot]?.notes ?? []).map((n) => ({
        ...n,
        reference: s.slot * 1000000 + n.reference,
        sourceReference: n.reference,
        sourceContentId: s.contentId,
        sourceHistoryId: s.id,
        sourceTitle: s.title,
        track: 0,
        index: 0,
        songTime: 0,
        at: 0,
        end: 0,
        offsetMs: null,
        referenceVelocity: 0,
        actualVelocity: null,
      })),
    ),
  };
  const more = async (s: ReviewSource) => {
    const version = epoch.current;
    setBusy(true);
    try {
      const old = loaded[s.slot];
      const data = await api.command<{ notes: Notes[]; total: number }>({
        type: "historyProblemNotes",
        content_id: s.contentId,
        id: s.id,
        offset: old.notes.length,
      });
      if (version !== epoch.current) return;
      setLoaded((v) => ({
        ...v,
        [s.slot]: {
          notes: [...old.notes, ...data.notes],
          total: data.total,
          error: "",
        },
      }));
    } catch (e) {
      if (version !== epoch.current) return;
      setLoaded((v) => ({
        ...v,
        [s.slot]: { ...v[s.slot], error: String(e) },
      }));
    } finally {
      if (version === epoch.current) setBusy(false);
    }
  };
  return (
    <details className="history-multi-sources">
      <summary>复习编排 · {sources.length}/20 条记录</summary>
      <p>
        先在历史详情中加入记录，再跨曲目选择问题音。每次最多 60
        个问题，沿用各曲原声部条件。
      </p>
      {sources.map((s) => (
        <div key={s.slot}>
          <span>
            {s.title} · {new Date(s.recordedAt).toLocaleString("zh-CN")} · 问题{" "}
            {loaded[s.slot]?.total ?? "读取中"}
          </span>
          <button disabled={busy} onClick={() => remove(s.id, s.contentId)}>
            移出复习编排
          </button>
          {loaded[s.slot]?.error && <p role="alert">{loaded[s.slot].error}</p>}
          {loaded[s.slot] &&
            loaded[s.slot].total > loaded[s.slot].notes.length && (
              <button disabled={busy} onClick={() => void more(s)}>
                再读取这条记录的 200 个问题
              </button>
            )}
        </div>
      ))}
      <HistoryProblemPlan
        multi
        review={review}
        disabled={busy || !sources.length}
        openPlan={openPlan}
        dirtyChanged={dirtyChanged}
      />
    </details>
  );
}
