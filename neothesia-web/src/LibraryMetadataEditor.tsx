import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { copyLibraryPaths } from "./LibraryContextMenu";
import type { Metadata } from "./libraryQuery";
export type MetadataEdit = { path: string; contentId: string; value: Metadata };
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
const equal = (a: unknown, b: unknown) =>
  JSON.stringify(a) === JSON.stringify(b);
const text = (value: unknown) =>
  Array.isArray(value) ? value.join("，") : String(value ?? "（空）");
export function LibraryMetadataEditor({
  edit,
  onBusy,
  saved,
  cancel,
}: {
  edit: MetadataEdit;
  onBusy: (busy: boolean) => void;
  saved: (warnings?: string[]) => Promise<void>;
  cancel: () => void;
}) {
  const [base, setBase] = useState(edit.value),
    [draft, setDraft] = useState(() => structuredClone(edit.value)),
    [conflict, setConflict] = useState<Metadata | null>(null),
    [choices, setChoices] = useState<
      Partial<Record<Field, "mine" | "latest" | "">>
    >({}),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState("");
  const working = useRef(false);
  const reviewRef = useRef<HTMLElement>(null);
  useEffect(() => {
    if (conflict) {
      reviewRef.current?.scrollIntoView({ block: "start" });
      reviewRef.current?.querySelector<HTMLSelectElement>("select")?.focus();
    }
  }, [conflict]);
  const present = (current: Metadata) => {
    const defaults: Partial<Record<Field, "mine" | "latest" | "">> = {};
    for (const [key] of fields)
      defaults[key] = equal(draft[key], current[key])
        ? "mine"
        : equal(draft[key], base[key])
          ? "latest"
          : equal(current[key], base[key])
            ? "mine"
            : "";
    setChoices(defaults);
    setConflict(current);
    setNotice("");
  };
  const job = async (action: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    onBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      setBusy(false);
      onBusy(false);
    }
  };
  const save = () =>
    void job(async () => {
      const reply = await api.command<{
        status: "saved" | "conflict";
        warnings?: string[];
        value?: Metadata;
        current?: Metadata;
      }>({
        type: "saveLibraryMetadata",
        path: edit.path,
        content_id: edit.contentId,
        value: draft,
        expected: base,
      });
      if (reply.status === "conflict") {
        present(reply.current!);
        return;
      }
      setBase(reply.value!);
      setDraft(reply.value!);
      setConflict(null);
      setNotice(`资料已保存。${reply.warnings?.length?reply.warnings.join("；"):""}`);
      try {
        await saved(reply.warnings);
      } catch (e) {
        throw new Error(
          `资料已保存，曲库刷新失败：${e instanceof Error ? e.message : String(e)}`,
        );
      }
    });
  const review = () =>
    void job(async () => {
      const result = await api.command<{ value: Metadata }>({
        type: "readLibraryMetadata",
        path: edit.path,
        content_id: edit.contentId,
      });
      if (equal(result.value, base)) {
        setConflict(null);
        setNotice("资料未变化，当前草稿保留。");
      } else present(result.value);
    });
  const changed = conflict
    ? fields.filter(([key]) => !equal(draft[key], conflict[key]))
    : [];
  const unresolved = changed.some(([key]) => !choices[key]);
  return (
    <section
      className="library-metadata-editor"
      aria-label="曲目信息编辑"
      aria-busy={busy}
    >
      <h4>编辑曲目信息</h4>
      <p className="file-path">{edit.path}</p>
      {fields.map(([key, label]) => (
        <label key={key}>
          {label}
          {key === "notes" ? (
            <textarea
              aria-label={`管理${label}`}
              rows={4}
              maxLength={10000}
              value={draft.notes ?? ""}
              disabled={busy || !!conflict}
              onChange={(e) =>
                setDraft({ ...draft, notes: e.target.value || null })
              }
            />
          ) : (
            <input
              aria-label={`管理${label}`}
              maxLength={key === "tags" ? 13000 : 2000}
              value={
                key === "tags" ? draft.tags.join("，") : (draft[key] ?? "")
              }
              disabled={busy || !!conflict}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  [key]:
                    key === "tags"
                      ? e.target.value
                          .split(/[,，]/)
                          .map((v) => v.trim())
                          .filter(Boolean)
                      : e.target.value || null,
                })
              }
            />
          )}
        </label>
      ))}
      {error && (
        <p role="alert" className="dialog-error">
          {error}
        </p>
      )}
      {notice && <p role="status">{notice}</p>}
      {conflict && (
        <section
          ref={reviewRef}
          className="metadata-conflicts"
          aria-label="曲目信息冲突审阅"
        >
          <h4>资料已在另一处修改，草稿尚未保存</h4>
          <p>不同字段默认保留各自修改；同字段双方修改时，请选择采用哪份。</p>
          <div className="metadata-conflict-table">
            <table>
              <thead>
                <tr>
                  <th>字段</th>
                  <th>打开时</th>
                  <th>我的草稿</th>
                  <th>最新资料</th>
                  <th>采用</th>
                </tr>
              </thead>
              <tbody>
                {changed.map(([key, label]) => (
                  <tr key={key}>
                    <th>
                      {label}
                      {!equal(draft[key], base[key]) &&
                        !equal(conflict[key], base[key]) && (
                          <small>双方修改</small>
                        )}
                    </th>
                    <td>{text(base[key])}</td>
                    <td>{text(draft[key])}</td>
                    <td>{text(conflict[key])}</td>
                    <td>
                      <select
                        aria-label={`冲突${label}采用`}
                        value={choices[key] ?? ""}
                        onChange={(e) =>
                          setChoices({
                            ...choices,
                            [key]: e.target.value as "mine" | "latest" | "",
                          })
                        }
                      >
                        <option value="">请选择</option>
                        <option value="mine">我的草稿</option>
                        <option value="latest">最新资料</option>
                      </select>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <button
            disabled={busy || unresolved}
            onClick={() => {
              const merged = structuredClone(conflict);
              for (const [key] of fields)
                if (choices[key] === "mine")
                  Object.assign(merged, { [key]: draft[key] });
              setBase(conflict);
              setDraft(merged);
              setConflict(null);
              setError("");
              setNotice(
                "已合并到草稿，请核对后保存；保存时会再次检查最新资料。",
              );
            }}
          >
            合并到草稿
          </button>
          <button
            disabled={busy}
            onClick={() => {
              setBase(conflict);
              setDraft(structuredClone(conflict));
              setConflict(null);
              setNotice("已采用最新资料，尚未写入任何修改。");
            }}
          >
            放弃草稿并采用最新资料
          </button>
        </section>
      )}
      <div className="file-actions">
        <button
          className="primary-button"
          disabled={busy || !!conflict}
          onClick={save}
        >
          保存曲目信息
        </button>
        <button disabled={busy} onClick={review}>
          读取最新资料并审阅
        </button>
        <button
          disabled={busy}
          onClick={() =>
            void job(async () => {
              await copyLibraryPaths([
                edit.path,
                ...fields.map(
                  ([key, label]) => `${label}：${text(draft[key])}`,
                ),
              ]);
              setNotice("已复制当前草稿。");
            })
          }
        >
          复制信息草稿
        </button>
        <button disabled={busy} onClick={cancel}>
          取消信息编辑
        </button>
      </div>
      <p className="parameter-help">
        保存或取消后再切换曲目或关闭曲库。保存只修改这些信息，指法、分手和谱面关联仍保留。
      </p>
    </section>
  );
}
