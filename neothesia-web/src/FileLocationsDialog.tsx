import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { api } from "./api";
type Kind = {
  id: string;
  label: string;
  paths: { path: string; available: boolean }[];
};
export function FileLocationsDialog({ close }: { close: () => void }) {
  const [kinds, setKinds] = useState<Kind[]>([]),
    [kind, setKind] = useState("music"),
    [busy, setBusy] = useState(true),
    [error, setError] = useState("");
  const previousFocus = useRef(document.activeElement as HTMLElement | null);
  useEffect(() => () => { if (previousFocus.current?.isConnected) previousFocus.current.focus(); }, []);
  const working = useRef(true);
  const load = async (command: unknown) => {
    working.current = true;
    setBusy(true);
    setError("");
    try {
      const value = (await api.command(
        command as Parameters<typeof api.command>[0],
      )) as { kinds: Kind[] };
      setKinds(value.kinds);
    } catch (e) {
      setError(String(e));
    } finally {
      working.current = false;
      setBusy(false);
    }
  };
  useEffect(() => {
    void load({ type: "fileLocations" });
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
  const selected = kinds.find((k) => k.id === kind);
  const update = (path: string | null, clear: boolean) =>
    void load({ type: "updateFileLocation", kind, path, clear });
  return (
    <div
      className="modal-backdrop file-locations-backdrop"
      onClick={(event) => {
        event.stopPropagation();
        if (event.target === event.currentTarget && !working.current) close();
      }}
    >
      <section
        className="settings-dialog file-locations-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="file-locations-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="file-locations-title">最近文件位置</h2>
            <p>文件选择窗口按用途记住最近使用的文件夹。</p>
          </div>
          <button
            autoFocus
            aria-label="关闭最近文件位置"
            disabled={busy}
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div className="file-locations-toolbar">
          <label>
            文件用途
            <select
              aria-label="文件位置用途"
              value={kind}
              disabled={busy}
              onChange={(e) => setKind(e.target.value)}
            >
              {kinds.map((k) => (
                <option key={k.id} value={k.id}>
                  {k.label}
                </option>
              ))}
            </select>
          </label>
          <button
            disabled={busy || !selected?.paths.length}
            onClick={() => update(null, true)}
          >
            清除此类记录
          </button>
          <button
            disabled={busy}
            onClick={() => void load({ type: "fileLocations" })}
          >
            刷新位置
          </button>
        </div>
        {error && <p role="alert">{error}</p>}
        <div className="file-locations-list" aria-busy={busy}>
          {selected?.paths.map((p, i) => (
            <div className="file-location-row" key={p.path}>
              <div>
                <strong>{i === 0 ? "下次选择位置" : "最近使用"}</strong>
                <small>{p.available ? "可用" : "当前不可用"}</small>
                <p title={p.path}>{p.path}</p>
              </div>
              <button
                disabled={busy || !p.available || i === 0}
                onClick={() => update(p.path, false)}
              >
                用于下次选择
              </button>
              <button
                disabled={busy}
                aria-label={`移除位置记录 ${p.path}`}
                onClick={() => update(p.path, true)}
              >
                移除记录
              </button>
            </div>
          ))}
          {!busy && !selected?.paths.length && (
            <p className="parameter-help">
              此类文件还没有位置记录。下次选择文件后会自动记住所在文件夹。
            </p>
          )}
        </div>
        <p className="parameter-help">
          每类保留最近 20
          个位置。文件夹断开或被移动时，选择窗口会打开仍可用的上级位置。移除记录只清理位置记忆，文件和曲库登记继续保留。
        </p>
      </section>
    </div>
  );
}
