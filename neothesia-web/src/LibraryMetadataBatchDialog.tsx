import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { api } from "./api";
import type { Metadata } from "./libraryQuery";
import {
  templateFields,
  emptyRules,
  modes,
  rulesError,
  metadataText,
  type Rules,
  type TemplateField,
} from "./metadataTemplates";
export type MetadataBatchItem = {
  path: string;
  contentId: string | null;
  title: string;
};
type Row = MetadataBatchItem & {
  status: "ready" | "unchanged" | "saved" | "conflict" | "error";
  include: boolean;
  base?: Metadata;
  value?: Metadata;
  message?: string;
  fields?: TemplateField[];
};
type Registry = {
  revision: number;
  templates: { name: string; rules: Partial<Rules> }[];
};
export function LibraryMetadataBatchDialog({
  items,
  close,
  changed,
}: {
  items: MetadataBatchItem[];
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [rules, setRules] = useState(emptyRules),
    [registry, setRegistry] = useState<Registry>({
      revision: 0,
      templates: [],
    }),
    [selected, setSelected] = useState(""),
    [name, setName] = useState(""),
    [rows, setRows] = useState<Row[]>([]),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [progress, setProgress] = useState("");
  const previousFocus = useRef(document.activeElement as HTMLElement | null);
  useEffect(() => () => { if (previousFocus.current?.isConnected) previousFocus.current.focus(); }, []);
  const working = useRef(false),
    stop = useRef(false),
    baseline = useRef("");
  const ready = rows.filter((r) => r.status === "ready" && r.include);
  const job = async (action: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    stop.current = false;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      working.current = false;
      setBusy(false);
      setProgress("");
    }
  };
  const loadTemplates = async () =>
    setRegistry(await api.command<Registry>({ type: "metadataTemplates" }));
  useEffect(() => {
    void job(loadTemplates);
  }, []);
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
  const changeRule = (
    field: TemplateField,
    change: Partial<Rules[TemplateField]>,
  ) => {
    setRules({ ...rules, [field]: { ...rules[field], ...change } });
    setRows([]);
    baseline.current = "";
    setNotice("");
  };
  const preview = () =>
    void job(async () => {
      const invalid = rulesError(rules);
      if (invalid) throw new Error(invalid);
      if (!items.length || items.length > 100)
        throw new Error("每批请选择 1 到 100 个曲目文件。");
      setRows([]);
      setNotice("");
      baseline.current = JSON.stringify(rules);
      const next: Row[] = [];
      for (const item of items) {
        if (stop.current) break;
        setProgress(`读取 ${next.length + 1} / ${items.length}：${item.title}`);
        try {
          const { base, value, contentId } = await api.command<{
            base: Metadata;
            value: Metadata;
            contentId: string;
          }>({
            type: "previewLibraryMetadata",
            rules,
            path: item.path,
            content_id: item.contentId,
          });
          const fields = templateFields
            .filter(
              ([field]) =>
                JSON.stringify(base[field]) !== JSON.stringify(value[field]),
            )
            .map(([field]) => field);
          next.push({
            ...item,
            contentId,
            base,
            value,
            fields,
            include: fields.length > 0,
            status: fields.length ? "ready" : "unchanged",
          });
        } catch (e) {
          next.push({
            ...item,
            status: "error",
            include: false,
            message: String(e),
          });
        }
        setRows([...next]);
      }
      setNotice(
        stop.current
          ? `已停止读取，保留 ${next.length} 个文件的预览。`
          : `已读取 ${next.length} 个文件；展开每行查看将修改的字段。`,
      );
    });
  const apply = () =>
    void job(async () => {
      if (baseline.current !== JSON.stringify(rules) || !ready.length)
        throw new Error("请先读取并检查当前字段的预览。");
      let next = [...rows],
        written = 0,
        processed = 0;
      setNotice("");
      for (const item of ready) {
        if (stop.current) break;
        setProgress(`保存 ${processed + 1} / ${ready.length}：${item.title}`);
        let result: Partial<Row>;
        try {
          const reply = await api.command<{ status: string; warnings?: string[] }>({
            type: "saveLibraryMetadata",
            path: item.path,
            content_id: item.contentId,
            value: item.value,
            expected: item.base,
          });
          if (reply.status === "conflict")
            result = {
              status: "conflict",
              include: false,
              message:
                "预览后资料已变化，尚未写入。请重新读取预览，检查最新字段。",
            };
          else if (reply.status === "saved") {
            written++;
            result = { status: "saved", include: false, message: `已保存${reply.warnings?.length?"；"+reply.warnings.join("；"):""}` };
          } else throw new Error("保存结果无法确认，请重新读取资料。");
        } catch (e) {
          result = { status: "error", include: false, message: String(e) };
        }
        next = next.map((row) =>
          row.path === item.path ? { ...row, ...result } : row,
        );
        setRows(next);
        processed++;
      }
      setNotice(
        `${stop.current ? "已停止；" : ""}已保存 ${written} 个文件，${processed - written} 个未完成。${ready.length - processed} 个仍待处理。`,
      );
      if (written) {
        try {
          await changed();
        } catch (e) {
          setError(`资料已保存，曲库刷新失败：${String(e)}`);
        }
      }
    });
  const saveTemplate = () =>
    void job(async () => {
      const invalid = rulesError(rules);
      if (invalid) throw new Error(invalid);
      const value = await api.command<Registry>({
        type: "updateMetadataTemplate",
        name,
        rules,
        expected: registry.revision,
      });
      setRegistry(value);
      setSelected(name.trim());
      setName(name.trim());
      setNotice("资料模板已保存。模板不包含曲名，也不包含任何文件路径。");
    });
  const removeTemplate = () =>
    void job(async () => {
      setRegistry(
        await api.command<Registry>({
          type: "updateMetadataTemplate",
          name: selected,
          rules: null,
          expected: registry.revision,
        }),
      );
      setSelected("");
      setNotice("模板已移除，当前字段设置保留。");
    });
  return (
    <div
      className="modal-backdrop metadata-batch-backdrop"
      onClick={(e) => {
        e.stopPropagation();
        if (e.target === e.currentTarget && !working.current) close();
      }}
    >
      <section
        className="settings-dialog metadata-batch-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="metadata-batch-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="metadata-batch-title">批量修改资料</h2>
            <p>
              已选 {items.length}{" "}
              个曲目文件。修改前逐首读取、预览，保存时再核对资料和曲目内容。
            </p>
          </div>
          <button
            autoFocus
            aria-label="关闭批量修改资料"
            disabled={busy}
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div className="metadata-template-toolbar">
          <label>
            资料模板
            <select
              aria-label="选择资料模板"
              value={selected}
              disabled={busy}
              onChange={(e) => {
                const name = e.target.value;
                setSelected(name);
                setName(name);
                const template = registry.templates.find(
                  (t) => t.name === name,
                );
                if (template) {
                  setRules({ ...emptyRules(), ...template.rules });
                  setRows([]);
                  baseline.current = "";
                  setNotice("模板已载入；请读取曲目预览。");
                }
              }}
            >
              <option value="">选择已保存模板</option>
              {registry.templates.map((t) => (
                <option key={t.name} value={t.name}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
          <input
            aria-label="资料模板名称"
            placeholder="模板名称"
            value={name}
            maxLength={80}
            disabled={busy}
            onChange={(e) => setName(e.target.value)}
          />
          <button disabled={busy || !name.trim()} onClick={saveTemplate}>
            {registry.templates.some((t) => t.name === name.trim())
              ? "更新模板"
              : "保存模板"}
          </button>
          <button disabled={busy || !selected} onClick={removeTemplate}>
            移除模板
          </button>
          <button disabled={busy} onClick={() => void job(loadTemplates)}>
            刷新模板
          </button>
        </div>
        <div className="metadata-batch-fields">
          {templateFields.map(([field, label]) => (
            <div key={field}>
              <label htmlFor={`batch-mode-${field}`}>{label}</label>
              <select
                id={`batch-mode-${field}`}
                aria-label={`批量${label}操作`}
                value={rules[field].mode}
                disabled={busy}
                onChange={(e) => changeRule(field, { mode: e.target.value })}
              >
                {modes(field).map(([mode, name]) => (
                  <option key={mode} value={mode}>
                    {name}
                  </option>
                ))}
              </select>
              {field === "notes" ? (
                <textarea
                  aria-label={`批量${label}内容`}
                  rows={2}
                  maxLength={10000}
                  disabled={
                    busy || ["keep", "clear"].includes(rules[field].mode)
                  }
                  value={rules[field].value}
                  onChange={(e) => changeRule(field, { value: e.target.value })}
                />
              ) : (
                <input
                  aria-label={`批量${label}内容`}
                  maxLength={field === "tags" ? 12900 : 2000}
                  disabled={
                    busy || ["keep", "clear"].includes(rules[field].mode)
                  }
                  placeholder={
                    field === "tags" ? "逗号或换行分隔标签" : "填写修改内容"
                  }
                  value={rules[field].value}
                  onChange={(e) => changeRule(field, { value: e.target.value })}
                />
              )}
            </div>
          ))}
        </div>
        <p className="parameter-help">
          “保留”不修改原字段；“清空”和“替换”会显示被移除的内容。追加标签去掉重复项，追加备注保留原文。每个副本的资料独立处理；曲名、指法、分手与谱面关联沿原资料保留。
        </p>
        <div className="metadata-batch-controls">
          <button
            disabled={busy || !items.length || items.length > 100}
            onClick={preview}
          >
            读取修改预览
          </button>
          <button disabled={busy || !ready.length} onClick={apply}>
            保存所选修改 · {ready.length}
          </button>
          {busy && progress && (
            <button
              onClick={() => {
                stop.current = true;
              }}
            >
              停止当前批次
            </button>
          )}
          <span role="status">{progress || notice}</span>
        </div>
        {error && <p role="alert">{error}</p>}
        <div className="metadata-batch-results" aria-busy={busy}>
          {rows.map((row) => (
            <article className="metadata-batch-row" key={row.path}>
              <div className="metadata-batch-row-heading">
                <input
                  type="checkbox"
                  aria-label={`修改资料 ${row.path}`}
                  checked={row.include}
                  disabled={busy || row.status !== "ready"}
                  onChange={(e) =>
                    setRows(
                      rows.map((r) =>
                        r.path === row.path
                          ? { ...r, include: e.target.checked }
                          : r,
                      ),
                    )
                  }
                />
                <strong>{row.base?.title || row.title}</strong>
                <span>
                  {
                    {
                      ready: "待保存",
                      unchanged: "无需修改",
                      saved: "已保存",
                      conflict: "资料冲突",
                      error: "未完成",
                    }[row.status]
                  }
                </span>
              </div>
              <p className="file-path">{row.path}</p>
              {row.message && <p>{row.message}</p>}
              {!!row.fields?.length && (
                <details>
                  <summary>查看 {row.fields.length} 个字段变化</summary>
                  <table>
                    <thead>
                      <tr>
                        <th>字段</th>
                        <th>原资料</th>
                        <th>修改后</th>
                      </tr>
                    </thead>
                    <tbody>
                      {row.fields.map((field) => (
                        <tr key={field}>
                          <th>
                            {templateFields.find(([key]) => key === field)?.[1]}
                          </th>
                          <td>{metadataText(row.base?.[field])}</td>
                          <td>{metadataText(row.value?.[field])}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </details>
              )}
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
