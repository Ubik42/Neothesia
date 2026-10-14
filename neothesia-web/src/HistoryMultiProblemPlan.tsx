import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { HistoryProblemPlan, type MissingProblemStatus } from "./HistoryProblemPlan";
import type { HistoryScoreReview } from "./historyScoreFocus";
import { draftScopeFor } from "./multiProblemDraft";
export type ReviewSource = {
  contentId: string;
  id: string;
  title: string;
  recordedAt: number;
  slot: number;
};
const sourceKey = (source: ReviewSource) =>
  JSON.stringify([source.contentId, source.id, source.slot]);
const changedSource = "演奏记录内容已变化。请重新读取，并核对草稿中选中的问题音。";
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
      Record<string, { notes: Notes[]; total: number; revision: string; error: string }>
    >({}),
    [busy, setBusy] = useState(false),
    [restoreMessage, setRestoreMessage] = useState("");
  const epoch = useRef(0);
  useEffect(() => {
    const version = ++epoch.current;
    let alive = true;
    setRestoreMessage("");
    setLoaded({});
    setBusy(true);
    void Promise.all(
      sources.map(async (s) => {
        try {
          const data = await api.command<{ notes: Notes[]; total: number; revision: string }>({
            type: "historyProblemNotes",
            content_id: s.contentId,
            id: s.id,
            offset: 0,
          });
          return [sourceKey(s), { ...data, error: "" }] as const;
        } catch (e) {
          return [sourceKey(s), { notes: [], total: 0, revision: "", error: String(e) }] as const;
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
      (loaded[sourceKey(s)]?.notes ?? []).map((n) => ({
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
  const missingStatus = (reference: number): MissingProblemStatus => {
    const source = sources.find((item) => item.slot === Math.floor(reference / 1_000_000));
    if (!source) return "gone";
    const row = loaded[sourceKey(source)];
    if (!row) return "loading";
    if (row.error) return "error";
    return row.notes.length < row.total ? "more" : "gone";
  };
  const more = async (s: ReviewSource) => {
    const version = epoch.current;
    const key = sourceKey(s);
    setBusy(true);
    try {
      const old = loaded[key];
      const reset = old.error === changedSource;
      const data = await api.command<{ notes: Notes[]; total: number; revision: string }>({
        type: "historyProblemNotes",
        content_id: s.contentId,
        id: s.id,
        offset: reset ? 0 : old.notes.length,
      });
      if (version !== epoch.current) return;
      if (!reset && old.revision && data.revision !== old.revision) {
        setLoaded((current) => ({ ...current, [key]: { ...old, error: changedSource } }));
        return;
      }
      setLoaded((v) => ({
        ...v,
        [key]: {
          notes: reset ? data.notes : [...old.notes, ...data.notes],
          total: data.total,
          revision: data.revision,
          error: !data.notes.length && (reset ? 0 : old.notes.length) < data.total
            ? "来源记录没有返回后续问题音，请重试读取。" : "",
        },
      }));
    } catch (e) {
      if (version !== epoch.current) return;
      setLoaded((v) => ({
        ...v,
        [key]: { ...v[key], error: String(e) },
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
        const key = sourceKey(source);
        let row = loaded[key];
        if (!row) continue;
        for (const note of row.notes) remaining.delete(source.slot * 1_000_000 + note.reference);
        const hasMissing = () => [...remaining].some((reference) =>
          Math.floor(reference / 1_000_000) === source.slot);
        while (hasMissing() && row.notes.length < row.total && batches < 20) {
          const data = await api.command<{ notes: Notes[]; total: number; revision: string }>({
            type: "historyProblemNotes",
            content_id: source.contentId,
            id: source.id,
            offset: row.notes.length,
          });
          if (version !== epoch.current) return;
          if (data.revision !== row.revision) {
            setLoaded((current) => ({ ...current, [key]: { ...row, error: changedSource } }));
            setRestoreMessage(changedSource);
            return;
          }
          if (!data.notes.length) {
            row = { ...row, total: data.total, error: row.notes.length < data.total
              ? "来源记录没有返回后续问题音，请重试读取。" : "" };
            const updated = row;
            setLoaded((current) => ({ ...current, [key]: updated }));
            break;
          }
          batches++;
          row = { notes: [...row.notes, ...data.notes], total: data.total, revision: row.revision, error: "" };
          const updated = row;
          setLoaded((current) => ({ ...current, [key]: updated }));
          for (const note of data.notes) remaining.delete(source.slot * 1_000_000 + note.reference);
          setRestoreMessage(`已继续读取 ${batches} 页问题音…`);
        }
      }
      if (version === epoch.current) setRestoreMessage(remaining.size
        ? `${remaining.size} 个已选问题音仍未找到。请查看下方状态；还有后续页时继续读取，原记录已变化时移除旧选择。`
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
            {loaded[sourceKey(s)]?.total ?? "读取中"}
          </span>
          <button disabled={busy} onClick={() => remove(s.id, s.contentId)}>
            移出复习编排
          </button>
          {loaded[sourceKey(s)]?.error && <p role="alert">{loaded[sourceKey(s)].error}</p>}
          {loaded[sourceKey(s)]?.error && (
            <button disabled={busy} onClick={() => void more(s)}>
              {loaded[sourceKey(s)].error === changedSource
                ? "重新读取已变化的记录"
                : "重试读取这条记录的问题音"}
            </button>
          )}
          {loaded[sourceKey(s)] && !loaded[sourceKey(s)].error &&
            loaded[sourceKey(s)].total > loaded[sourceKey(s)].notes.length && (
              <button disabled={busy} onClick={() => void more(s)}>
                再读取这条记录的 200 个问题
              </button>
            )}
        </div>
      ))}
      {sources.length > 0 && sources.every((source) => loaded[sourceKey(source)]) && <HistoryProblemPlan
        key={JSON.stringify(sources.map(sourceKey))}
        multi
        draftScope={draftScope}
        draftSources={sources.map((source) => ({ ...source, revision: loaded[sourceKey(source)].revision }))}
        onRestoreMissing={restoreMissing}
        missingStatus={missingStatus}
        restoreMessage={restoreMessage}
        review={review}
        disabled={busy || sources.some((source) => !!loaded[sourceKey(source)].error)}
        openPlan={openPlan}
        dirtyChanged={dirtyChanged}
      />}
    </details>
  );
}
