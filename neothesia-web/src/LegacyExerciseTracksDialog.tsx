import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { api } from "./api";

type Review = {
  contentId: string;
  fingerprint: string;
  sourcePath: string;
  activePath: string;
  fromTrack: number;
  toTrack: number;
  sourceAnnotations: number;
  targetAnnotations: number;
  savedConditionTracks: number;
  recentHistoryTracks: number;
  warnings: string[];
  canApply: boolean;
  historyPolicy: string;
  firstLoadSavedConditions: boolean;
  firstLoadHistorySetup: boolean;
  conditionConflict: boolean;
};

export function LegacyExerciseTracksDialog({
  contentId,
  close,
  changed,
}: {
  contentId: string;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [review, setReview] = useState<Review | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [restoreConditions, setRestoreConditions] = useState(false);
  const load = async () => {
    setBusy(true);
    setError("");
    try {
      setReview(await api.command<Review>({ type: "previewLegacyExerciseTracks", content_id: contentId }));
    } catch (e) {
      setReview(null);
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => { void load(); }, [contentId]);
  const apply = async () => {
    if (!review?.canApply) return;
    setBusy(true);
    setError("");
    try {
      const result = await api.command<{ backupPath: string; migratedAnnotations: number }>({
        type: "applyLegacyExerciseTracks",
        content_id: contentId,
        fingerprint: review.fingerprint,
        restore_conditions: restoreConditions,
      });
      await changed();
      setNotice(`已迁移 ${result.migratedAnnotations} 项个人设置，并重新打开当前练习。原资料副本：${result.backupPath}`);
      setReview(null);
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
      setReview(null);
    } finally {
      setBusy(false);
    }
  };
  return <div className="modal-backdrop" onClick={close}>
    <section className="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="legacy-track-title" onClick={e => e.stopPropagation()}>
      <div className="dialog-heading">
        <h2 id="legacy-track-title">旧版左手练习资料核对</h2>
        <button aria-label="关闭旧版资料核对" onClick={close}><X size={18} /></button>
      </div>
      <p>旧版单左手技术练习曾使用 1 号音轨。当前版本使用 2 号音轨。软件会核对保留的旧 MIDI 音符，再由你确认个人设置是否转到新音轨。</p>
      {review && <>
        <p>旧 1 号轨个人设置：{review.sourceAnnotations} 项；新 2 号轨已有设置：{review.targetAnnotations} 项。</p>
        <p>保存的练习条件含旧轨：{review.savedConditionTracks} 项；最近历史设置含旧轨：{review.recentHistoryTracks} 项。</p>
        {(review.firstLoadSavedConditions || review.firstLoadHistorySetup) && <label>
          <input type="checkbox" disabled={review.conditionConflict} checked={restoreConditions} onChange={e => setRestoreConditions(e.target.checked)} />
          恢复首次载入前保存的练习条件（会替换当前这首练习的最近条件）
        </label>}
        {review.conditionConflict && <p className="form-error">旧练习条件同时含有两个音轨或手别冲突，软件会保留原设置供单独审阅。</p>}
        <p>旧 MIDI：{review.sourcePath}</p>
        <p>当前 MIDI：{review.activePath}</p>
        <p>{review.historyPolicy}</p>
        {review.warnings.map((warning, index) => <p className="form-error" key={index}>{warning}</p>)}
        <button disabled={busy || !review.canApply} onClick={() => void apply()}>确认迁移个人设置</button>
      </>}
      {error && <p className="form-error" role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      <button disabled={busy} onClick={() => void load()}>重新检查</button>
    </section>
  </div>;
}
