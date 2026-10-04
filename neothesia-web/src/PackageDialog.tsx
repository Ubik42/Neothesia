import { useEffect, useRef, useState } from "react";
import { X, Package, Download, FolderOpen } from "lucide-react";
import { api, type LoadedSong } from "./api";
type Summary = {
  token: string;
  papers: {
    name: string;
    format: string;
    pages: number;
    mappingPoints?: number;
    autoTurn?: boolean;
    annotations?: number;
  }[];
  ladders: string[];
  title: string;
  contentId: string;
  metadata: { composer?: string; notes?: string; tags: string[] };
  scores: string[];
  fingerings: number;
  fingerActions?: number;
  trackSounds?: number;
  trackAppearances?: { id: number; name: string; color: string | null }[];
  hands: number;
  passages: string[];
  groups: string[][];
  rating: number;
  existing?: boolean;
  provenance?: { source?: string; license?: string; sourceUrl?: string };
};
export function PackageDialog({
  song,
  close,
  changed,
  initialFile,
}: {
  song: LoadedSong | null;
  close: () => void;
  changed: () => Promise<void>;
  initialFile?: File;
}) {
  const [summary, setSummary] = useState<Summary | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState(""),
    [policy, setPolicy] = useState("merge"),
    [fileName, setFileName] = useState("");
  const token = useRef<string | null>(null),
    input = useRef<HTMLInputElement>(null),
    started = useRef(false);
  const inspect = async (file: File) => {
    setBusy(true);
    setError("");
    setMessage("");
    setSummary(null);
    token.current = null;
    setFileName(file.name);
    try {
      const result = (await api.stagePackage(file)) as Summary;
      token.current = result.token;
      setSummary(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  // Only the initial drop starts inspection automatically. Import remains explicit.
  useEffect(() => {
    if (initialFile && !started.current) {
      started.current = true;
      void inspect(initialFile);
    }
  }, [initialFile]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !busy) close();
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [busy, close]);
  const exportPackage = async () => {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      if (api.desktop) {
        const path = await api.exportPackage();
        if (path) setMessage("曲目包已保存：" + path);
      } else {
        const url = URL.createObjectURL(await api.downloadPackage());
        const a = document.createElement("a");
        a.href = url;
        a.download =
          (song?.title || "曲目").replace(/[<>:"/\\|?*]/g, "_") + ".neopiece";
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        setMessage("曲目包已导出");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const importPackage = async () => {
    if (!token.current) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await api.command({
        type: "importStagedPackage",
        token: token.current,
        policy,
      });
      await changed();
      setMessage("曲目包已导入，可以关闭窗口开始练习");
      token.current = null;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog fingering-dialog package-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="package-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="package-title">曲目包</h2>
            <p>MIDI、谱页、教学标注与练习规则一起保存</p>
          </div>
          <button
            autoFocus
            aria-label="关闭曲目包"
            disabled={busy}
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div className="package-export">
          <Package size={22} />
          <div>
            <strong>{song?.title || "尚未打开曲目"}</strong>
            <p>
              包含全部谱面版本、元数据、指法、分手、拍号修正、分组评级、手型设置、命名练习段落、速度阶梯及音轨名称、颜色与声音设置。PDF
              与图片包含页序及阅读设置。个人成绩、阶梯进度及设备设置保留在本机。导出会暂停当前练习。
            </p>
          </div>
          <button disabled={!song || busy} onClick={() => void exportPackage()}>
            <Download size={16} />
            导出当前曲目包
          </button>
        </div>
        <div
          className="import-drop"
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (!busy && e.dataTransfer.files[0])
              void inspect(e.dataTransfer.files[0]);
          }}
        >
          <FolderOpen size={24} />
          <span>拖入一个 .neopiece 曲目包，先查看内容</span>
          <button
            disabled={busy}
            onClick={() => {
              if (!api.desktop) {
                input.current?.click();
                return;
              }
              setBusy(true);
              setError("");
              setSummary(null);
              token.current = null;
              void api
                .pickPackage()
                .then((v) => {
                  if (v) {
                    const result = v as unknown as Summary;
                    token.current = result.token;
                    setSummary(result);
                    setFileName(String(v.fileName || "曲目包"));
                  }
                })
                .catch((e) => setError(String(e)))
                .finally(() => setBusy(false));
            }}
          >
            选择曲目包
          </button>
          <input
            hidden
            ref={input}
            type="file"
            accept=".neopiece"
            aria-label="导入曲目包文件"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) void inspect(f);
              e.target.value = "";
            }}
          />
        </div>
        {busy && <p role="status">正在处理曲目包…</p>}
        {summary && (
          <div className="package-preview">
            <h3>{summary.title}</h3>
            {!!summary.fingerActions && <p>持音换指 {summary.fingerActions} 次；合并时保留已有整套换指动作，重新核对当前指法后提示。</p>}
            <p className="parameter-help">
              {fileName} · {summary.existing ? "本地已有同一内容" : "新增曲目"}
            </p>
            <dl>
              <dt>作曲家</dt>
              <dd>{summary.metadata.composer || "未填写"}</dd>
              <dt>谱面版本</dt>
              <dd>
                {summary.scores.length ? summary.scores.join("、") : "无"}
              </dd>
              <dt>PDF 与图片谱页</dt>
              <dd>
                {summary.papers
                  ?.map(
                    (p) =>
                      `${p.name}（${p.format === "pdf" ? "PDF" : `${p.pages} 页图片`}${p.mappingPoints ? ` · ${p.mappingPoints} 处小节对应${p.autoTurn ? " · 自动翻页" : ""}` : ""}${p.annotations ? ` · ${p.annotations} 条批注` : ""}）`,
                  )
                  .join("、") || "无"}
              </dd>
              <dt>速度阶梯</dt>
              <dd>{summary.ladders?.join("、") || "无"}</dd>
              <dt>指法与分手</dt>
              <dd>
                {summary.fingerings} 个指法 · {summary.hands} 个逐音分手
              </dd>
              <dt>音轨设置</dt>
              <dd className="package-track-settings">
                {summary.trackSounds
                  ? `${summary.trackSounds} 个声音设置`
                  : "无自选声音"}
                {summary.trackAppearances?.map((t) => (
                  <span key={t.id}>
                    <i
                      className="track-color-dot"
                      style={{ background: t.color ?? "#8591a3" }}
                    />
                    {t.name}
                  </span>
                ))}
              </dd>
              <dt>练习段落</dt>
              <dd>
                {summary.passages.length ? summary.passages.join("、") : "无"}
              </dd>
              <dt>分组与评级</dt>
              <dd>
                {summary.groups.map((g) => g.join(" / ")).join("、") ||
                  "未分组"}{" "}
                · {summary.rating} 星
              </dd>
              <dt>来源与许可</dt>
              <dd>
                {[summary.provenance?.source, summary.provenance?.license]
                  .filter(Boolean)
                  .join(" · ") || "未提供"}
              </dd>
              {summary.metadata.notes && (
                <>
                  <dt>备注</dt>
                  <dd>{summary.metadata.notes}</dd>
                </>
              )}
            </dl>
            <label>
              遇到已有曲目
              <select
                aria-label="曲目包导入策略"
                value={policy}
                disabled={busy}
                onChange={(e) => setPolicy(e.target.value)}
              >
                <option value="merge">合并，保留本地冲突项</option>
                <option value="keep">保留本地标注与练习设置</option>
                <option value="replace">使用包内标注与练习设置</option>
              </select>
            </label>
            <p className="parameter-help">
              {policy === "merge"
                ? "补充本地缺少的标注、谱页、练习段落和速度阶梯；已有指法、分手和备注优先保留。"
                : policy === "keep"
                  ? "已有标注与段落保持不变，补充包内谱面版本与谱页，保留本地阅读位置。"
                  : "替换当前曲目的标注、分组评级和练习设置；已有成绩与谱面文件保留；谱页目录、阅读设置和速度阶梯采用包内内容，阶梯进度重新开始。"}
            </p>
            <button
              className="primary"
              disabled={busy || !token.current}
              onClick={() => void importPackage()}
            >
              确认导入曲目包
            </button>
          </div>
        )}
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        {message && (
          <p className="package-message" role="status">
            {message}
          </p>
        )}
      </section>
    </div>
  );
}
