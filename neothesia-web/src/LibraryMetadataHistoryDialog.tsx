import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { api } from "./api";
import type { Metadata } from "./libraryQuery";
const fields = [
  ["title", "曲名"],
  ["composer", "作曲家"],
  ["artist", "演奏者"],
  ["collection", "作品集"],
  ["difficulty", "难度"],
  ["tags", "标签"],
  ["notes", "备注"],
] as const;
type Field = (typeof fields)[number][0];
type HistoryRecord = {
  id: string;
  timeMs: number;
  before: Metadata;
  after: Metadata;
  state: string;
  restoredFrom: string | null;
  fields: Field[];
  matchesCurrent: boolean;
};
type History = {
  records: HistoryRecord[];
  retention: number;
  available: boolean;
  error: string | null;
};
type Preview = {
  base: Metadata;
  value: Metadata;
  after: Metadata;
  fields: Field[];
  conflicts: Field[];
};
const text = (value: unknown) =>
  Array.isArray(value) ? value.join("，") : String(value ?? "（空）");
export type MetadataHistoryContext = {
  path: string;
  contentId: string;
  title: string;
};
export function LibraryMetadataHistoryDialog({
  path,
  contentId,
  title,
  close,
  changed,
}: MetadataHistoryContext & {
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [history, setHistory] = useState<History>({
      records: [],
      retention: 200,
      available: false,
      error: null,
    }),
    [selected, setSelected] = useState(""),
    [preview, setPreview] = useState<Preview | null>(null),
    [choices, setChoices] = useState<
      Partial<Record<Field, "restore" | "keep" | "">>
    >({}),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState("");
  const working = useRef(false),
    previousFocus = useRef(document.activeElement as HTMLElement | null);
  useEffect(
    () => () => {
      if (previousFocus.current?.isConnected) previousFocus.current.focus();
    },
    [],
  );
  const job = async (action: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      working.current = false;
      setBusy(false);
    }
  };
  const load = async () => {
    const value = await api.command<History>({
      type: "libraryMetadataHistory",
      path,
      content_id: contentId,
    });
    setHistory(value);
    return value;
  };
  useEffect(() => {
    void job(async () => {await load();});
  }, [path, contentId]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopImmediatePropagation();
        event.preventDefault();
        if (!working.current) close();
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [close]);
  const record = history.records.find((r) => r.id === selected);
  const selectedFields =
    preview?.fields.filter((field) => choices[field] === "restore") ?? [];
  const unresolved = preview?.conflicts.some((field) => !choices[field]);
  const readPreview = () =>
    void job(async () => {
      setNotice("");
      const value = await api.command<Preview>({
        type: "previewMetadataRestore",
        path,
        content_id: contentId,
        id: selected,
      });
      setPreview(value);
      setChoices(
        Object.fromEntries(
          value.fields.map((field) => [
            field,
            value.conflicts.includes(field) ? "" : "restore",
          ]),
        ),
      );
    });
  const restore = () =>
    void job(async () => {
      if (!preview || unresolved || !selectedFields.length)
        throw new Error("请先检查恢复预览并选择字段。");
      const value = await api.command<{ status: string; warnings?: string[] }>({
        type: "restoreLibraryMetadata",
        path,
        content_id: contentId,
        id: selected,
        fields: selectedFields,
        expected: preview.base,
      });
      if (value.status === "conflict") {
        setPreview(null);
        setNotice(
          "预览后资料再次变化，尚未恢复。请重新读取恢复预览并检查最新内容。",
        );
        return;
      }
      if (value.status !== "saved")
        throw new Error("恢复结果无法确认，请刷新查看当前资料。");
      setPreview(null);
      setChoices({});
      setNotice(
        `已恢复所选字段。此操作已记入新的资料历史。${value.warnings?.length ? value.warnings.join("；") : ""}`,
      );
      await load();
      try {
        await changed();
      } catch (e) {
        setError(`资料已恢复，曲库刷新失败：${String(e)}`);
      }
    });
  return (
    <div
      className="modal-backdrop metadata-history-backdrop"
      onClick={(e) => {
        e.stopPropagation();
        if (e.target === e.currentTarget && !working.current) close();
      }}
    >
      <section
        className="settings-dialog metadata-history-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="metadata-history-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="metadata-history-title">资料变更历史</h2>
            <p>{title}</p>
            <p className="file-path">{path}</p>
          </div>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭资料变更历史"
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div className="metadata-history-toolbar">
          <button
            disabled={busy}
            onClick={() =>
              void job(async () => {
                await load();
                setPreview(null);
                setNotice("历史已刷新，请重新读取恢复预览。");
              })
            }
          >
            刷新资料历史
          </button>
          <span>
            每个文件保留最近 {history.retention}{" "}
            次写入；当前内容身份的记录列在此处。
          </span>
        </div>
        {history.error && (
          <p className="dialog-error">
            {history.error}。历史仍可查看，恢复需要同一内容的可用文件。
          </p>
        )}
        {error && <p role="alert">{error}</p>}
        {notice && <p role="status">{notice}</p>}
        <div className="metadata-history-workspace">
          <div className="metadata-history-list" aria-label="资料历史列表">
            {!busy && !history.records.length && (
              <p>
                此文件尚无资料变更记录。单首编辑和批量资料修改保存后会开始记录。
              </p>
            )}
            {history.records.map((r) => (
              <button
                key={r.id}
                className={selected === r.id ? "active" : ""}
                disabled={busy}
                onClick={() => {
                  setSelected(r.id);
                  setPreview(null);
                  setChoices({});
                  setNotice("");
                }}
              >
                <strong>
                  {r.restoredFrom ? "恢复资料" : "修改资料"} ·{" "}
                  {new Date(r.timeMs).toLocaleString("zh-CN", {
                    hour12: false,
                  })}
                </strong>
                <span>
                  {r.fields
                    .map((field) => fields.find(([key]) => key === field)?.[1])
                    .join("、")}
                </span>
                <small>
                  {r.state === "applied"
                    ? r.matchesCurrent
                      ? "与当前全部资料一致"
                      : "已记录"
                    : r.state === "failed"
                      ? "写入失败"
                      : "写入状态未确认"}
                </small>
              </button>
            ))}
          </div>
          <div className="metadata-history-detail">
            {!record ? (
              <p>选择一条记录查看具体字段变化。</p>
            ) : (
              <>
                <h3>
                  {record.restoredFrom
                    ? "恢复操作的字段变化"
                    : "此次修改的字段变化"}
                </h3>
                <table>
                  <thead>
                    <tr>
                      <th>字段</th>
                      <th>修改前</th>
                      <th>修改后</th>
                    </tr>
                  </thead>
                  <tbody>
                    {record.fields.map((field) => (
                      <tr key={field}>
                        <th>{fields.find(([key]) => key === field)?.[1]}</th>
                        <td>{text(record.before[field])}</td>
                        <td>{text(record.after[field])}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {record.state !== "applied" && (
                  <p className="parameter-help">
                    这次写入未确认或失败，不能直接作为恢复来源；仍可查看当时准备写入的内容。
                  </p>
                )}
                <button
                  disabled={
                    busy || !history.available || record.state !== "applied"
                  }
                  onClick={readPreview}
                >
                  预览恢复到修改前
                </button>
              </>
            )}
            {preview && (
              <section aria-label="资料恢复审阅">
                <h3>恢复审阅</h3>
                <p>
                  只恢复选定字段，其他最新资料保留。同一字段后来改过时，请明确选择。
                </p>
                <table>
                  <thead>
                    <tr>
                      <th>字段</th>
                      <th>这次修改后</th>
                      <th>当前最新</th>
                      <th>恢复目标</th>
                      <th>本次操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {preview.fields.map((field) => (
                      <tr key={field}>
                        <th>
                          {fields.find(([key]) => key === field)?.[1]}
                          {preview.conflicts.includes(field) && (
                            <small>后来改过</small>
                          )}
                        </th>
                        <td>{text(preview.after[field])}</td>
                        <td>{text(preview.base[field])}</td>
                        <td>{text(preview.value[field])}</td>
                        <td>
                          <select
                            aria-label={`恢复${fields.find(([key]) => key === field)?.[1]}操作`}
                            disabled={busy}
                            value={choices[field] ?? ""}
                            onChange={(e) =>
                              setChoices({
                                ...choices,
                                [field]: e.target.value as
                                  "restore" | "keep" | "",
                              })
                            }
                          >
                            <option value="">请选择</option>
                            <option value="restore">恢复目标值</option>
                            <option value="keep">保留最新值</option>
                          </select>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <button
                  disabled={busy || !!unresolved || !selectedFields.length}
                  onClick={restore}
                >
                  恢复所选字段 · {selectedFields.length}
                </button>
              </section>
            )}
          </div>
        </div>
        <p className="parameter-help">
          仅记录本功能启用后的单首/批量资料保存和恢复，导入与外部程序改动不补记为已知历史。恢复只修改资料字段，指法、分手和谱面关联保留。历史保存在应用数据中，当前尚不随练习备份携带。
        </p>
      </section>
    </div>
  );
}
