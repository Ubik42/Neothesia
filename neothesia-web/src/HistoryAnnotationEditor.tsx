import { useEffect, useState } from "react";
export interface Annotation {
  note: string;
  teacher: string;
  next: string;
  updated_at_unix_ms: number;
}
export function HistoryAnnotationEditor({
  value,
  locked,
  onLocked,
  save,
}: {
  value: Annotation | null;
  locked: boolean;
  onLocked: (v: boolean) => void;
  save: (
    draft: { note: string; teacher: string; next: string },
    expected: Annotation | null,
  ) => Promise<void>;
}) {
  const initial = () => ({
    note: value?.note ?? "",
    teacher: value?.teacher ?? "",
    next: value?.next ?? "",
  });
  const [draft, setDraft] = useState(initial),
    [saving, setSaving] = useState(false),
    [error, setError] = useState("");
  const dirty =
    draft.note !== (value?.note ?? "") ||
    draft.teacher !== (value?.teacher ?? "") ||
    draft.next !== (value?.next ?? "");
  useEffect(() => {
    setDraft(initial());
    setError("");
  }, [value]);
  useEffect(() => {
    onLocked(dirty || saving);
    return () => onLocked(false);
  }, [dirty, saving, onLocked]);
  return (
    <details className="history-annotation" open>
      <summary>备注与教师评语{value && " · 已保存"}</summary>
      {[
        ["note", "练习备注"],
        ["teacher", "教师评语"],
        ["next", "下次重点"],
      ].map(([key, label]) => (
        <label key={key}>
          {label}
          <textarea
            aria-label={label}
            rows={2}
            maxLength={2048}
            value={draft[key as keyof typeof draft]}
            disabled={saving || locked}
            onChange={(e) => setDraft((v) => ({ ...v, [key]: e.target.value }))}
          />
        </label>
      ))}
      <div className="history-annotation-actions">
        <button
          disabled={!dirty || saving || locked}
          onClick={() => {
            setSaving(true);
            setError("");
            void save(draft, value)
              .catch((e) => setError(String(e.message ?? e)))
              .finally(() => setSaving(false));
          }}
        >
          保存评语
        </button>
        <button
          disabled={!dirty || saving || locked}
          onClick={() => {
            setDraft(initial());
            setError("");
          }}
        >
          放弃修改
        </button>
        {dirty && <span role="status">尚未保存，请保存或放弃后切换记录</span>}
      </div>
      {value && (
        <p className="parameter-help">
          更新于 {new Date(value.updated_at_unix_ms).toLocaleString("zh-CN")}
          。文字评语不改变成绩与达标判断。
        </p>
      )}
      {error && <p role="alert">{error}</p>}
    </details>
  );
}
