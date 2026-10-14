import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { HistoryProblemPlan } from "./HistoryProblemPlan";
import type { HistoryScoreReview } from "./historyScoreFocus";
import { draftScopeFor } from "./multiProblemDraft";
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
  velocityDifference?: number;
  velocityLimit?: number;
  timingKind?: string | null;
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
    [busy, setBusy] = useState(false),
    [restoreMessage, setRestoreMessage] = useState("");
  const epoch = useRef(0);
  useEffect(() => {
    const version = ++epoch.current;
    let alive = true;
    setRestoreMessage("");
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
  const draftScope = draftScopeFor(sources);
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
  const restoreMissing = async (references: number[]) => {
    const version = epoch.current;
    const remaining = new Set(references);
    let batches = 0;
    setBusy(true);
    setRestoreMessage("正在读取草稿中已选的问题音…");
    try {
      for (const source of sources) {
        if (version !== epoch.current || batches >= 20) break;
        let row = loaded[source.slot];
        if (!row) continue;
        for (const note of row.notes) remaining.delete(source.slot * 1_000_000 + note.reference);
        const hasMissing = () => [...remaining].some((reference) =>
          Math.floor(reference / 1_000_000) === source.slot);
        while (hasMissing() && row.notes.length < row.total && batches < 20) {
          const data = await api.command<{ notes: Notes[]; total: number }>({
            type: "historyProblemNotes",
            content_id: source.contentId,
            id: source.id,
            offset: row.notes.length,
          });
          if (version !== epoch.current) return;
          if (!data.notes.length) break;
          batches++;
          row = { notes: [...row.notes, ...data.notes], total: data.total, error: "" };
          const updated = row;
          setLoaded((current) => ({ ...current, [source.slot]: updated }));
          for (const note of data.notes) remaining.delete(source.slot * 1_000_000 + note.reference);
          setRestoreMessage(`已继续读取 ${batches} 页问题音…`);
        }
      }
      if (version === epoch.current) setRestoreMessage(remaining.size
        ? `${remaining.size} 个已选问题音仍未找到。若这条记录还有后续问题，请再次点击“继续读取已选问题音”；记录已变化时请移除旧选择。`
        : "草稿中已选的问题音已全部读取。");
    } catch (cause) {
      if (version === epoch.current) setRestoreMessage(cause instanceof Error ? cause.message : String(cause));
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
        key={draftScope}
        multi
        draftScope={draftScope}
        draftSources={sources}
        onRestoreMissing={restoreMissing}
        restoreMessage={restoreMessage}
        review={review}
        disabled={busy || !sources.length}
        openPlan={openPlan}
        dirtyChanged={dirtyChanged}
      />
    </details>
  );
}
