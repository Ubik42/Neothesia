import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import type { HistoryReplay } from "./useHistoryReplay";
type Clip = {
  id: string;
  name: string;
  start: number;
  end: number;
  notes: string;
  updatedAtUnixMs: number;
};
type Rows = { clips: Clip[]; revision: string };
export function HistoryReplayClips({
  replay,
  start,
  end,
  pick,
}: {
  replay: HistoryReplay;
  start: number;
  end: number;
  pick: (start: number, end: number) => void;
}) {
  const key = `replay-clip-draft:${replay.contentId}:${replay.historyId}`;
  const [rows, setRows] = useState<Rows | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [search, setSearch] = useState(""),
    [limit, setLimit] = useState(20),
    [remove, setRemove] = useState<string | null>(null);
  const [draft, setDraft] = useState<{
    id: string | null;
    name: string;
    notes: string;
  }>(() => {
    try {
      return (
        JSON.parse(sessionStorage.getItem(key) || "null") || {
          id: null,
          name: "",
          notes: "",
        }
      );
    } catch {
      return { id: null, name: "", notes: "" };
    }
  });
  const alive = useRef(true),
    working = useRef(false),
    revision = useRef(0);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      revision.current++;
    };
  }, []);
  useEffect(() => {
    try {
      if (draft.name || draft.notes || draft.id)
        sessionStorage.setItem(key, JSON.stringify(draft));
      else sessionStorage.removeItem(key);
    } catch {}
  }, [draft, key]);
  const scope = { content_id: replay.contentId, id: replay.historyId };
  async function work(job: () => Promise<Rows>, clear = false) {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    const request = ++revision.current;
    try {
      const data = await job();
      if (!alive.current || request !== revision.current) return;
      setRows(data);
      setRemove(null);
      if (clear) setDraft({ id: null, name: "", notes: "" });
    } catch (e) {
      if (alive.current && request === revision.current)
        setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  }
  const reload = () =>
    work(() => api.command<Rows>({ type: "historyReplayClips", ...scope }));
  useEffect(() => {
    void reload();
  }, []);
  const valid =
    Number.isFinite(start) &&
    Number.isFinite(end) &&
    start >= 0 &&
    end - start >= 0.1 &&
    end <= replay.end + 0.001;
  const filtered = (rows?.clips ?? []).filter((c) =>
    `${c.name} ${c.notes}`
      .toLocaleLowerCase()
      .includes(search.toLocaleLowerCase()),
  );
  async function locate(c: Clip) {
    if (replay.loop)
      await replay.control("loop", 0, { end: replay.end, enabled: false });
    if (alive.current) await replay.control("seek", c.start);
  }
  return (
    <details className="history-replay-clips">
      <summary>收藏的回放片段{rows ? ` · ${rows.clips.length}` : ""}</summary>
      <p>
        按这次演奏收藏上方选段。开始回放后可定位或循环听。
      </p>
      <div className="history-range-controls">
        <label>
          片段名称{" "}
          <input
            aria-label="回放片段名称"
            maxLength={80}
            value={draft.name}
            disabled={busy}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
          />
        </label>
        <span>
          {start.toFixed(3)}–{end.toFixed(3)} 秒
        </span>
        <button
          disabled={
            busy || !rows || !valid || !draft.name.trim() || replay.busy
          }
          onClick={() =>
            void work(
              () =>
                api.command<Rows>({
                  type: "saveHistoryReplayClip",
                  ...scope,
                  clip_id: draft.id,
                  name: draft.name,
                  notes: draft.notes,
                  start,
                  end,
                  expected: rows!.revision,
                }),
              true,
            )
          }
        >
          {draft.id ? "保存片段修改" : "收藏当前范围"}
        </button>
        {draft.id && (
          <button
            disabled={busy}
            onClick={() => setDraft({ id: null, name: "", notes: "" })}
          >
            取消修改
          </button>
        )}
      </div>
      <label className="history-clip-notes">
        片段备注
        <textarea
          aria-label="回放片段备注"
          maxLength={2000}
          disabled={busy}
          value={draft.notes}
          onChange={(e) => setDraft({ ...draft, notes: e.target.value })}
        />
      </label>
      {(draft.name || draft.notes || draft.id) && (
        <p>未保存的名称和备注暂留在本窗口。起点、终点以上方所选范围为准。</p>
      )}
      {error && (
        <p role="alert">
          {error}。重新读取会更新收藏列表，并保留输入的名称和备注。
        </p>
      )}
      <div className="history-range-controls">
        <label>
          查找片段{" "}
          <input
            aria-label="查找回放片段"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setLimit(20);
            }}
          />
        </label>
        <button disabled={busy} onClick={() => void reload()}>
          重新读取片段
        </button>
      </div>
      {rows && !rows.clips.length && (
        <p>还没有收藏片段。先采用小节范围，或设置回放起点和终点。</p>
      )}
      {rows && rows.clips.length > 0 && !filtered.length && (
        <p>没有匹配的片段。</p>
      )}
      <ul className="history-clip-list">
        {filtered.slice(0, limit).map((c) => (
          <li key={c.id}>
            <div>
              <strong>{c.name}</strong>
              <span>
                {c.start.toFixed(3)}–{c.end.toFixed(3)} 秒
              </span>
            </div>
            {c.notes && <p>{c.notes}</p>}
            <div className="history-range-controls">
              <button
                disabled={busy || replay.busy}
                onClick={() => pick(c.start, c.end)}
              >
                采用片段范围
              </button>
              <button
                disabled={busy || replay.busy || replay.phase === "idle"}
                onClick={() => void locate(c)}
              >
                定位片段起点
              </button>
              <button
                disabled={busy}
                onClick={() => {
                  pick(c.start, c.end);
                  setDraft({ id: c.id, name: c.name, notes: c.notes });
                  setRemove(null);
                }}
              >
                修改片段
              </button>
              {remove === c.id ? (
                <>
                  <span>删除这个收藏？</span>
                  <button
                    disabled={busy || !rows}
                    onClick={() =>
                      void work(
                        () =>
                          api.command<Rows>({
                            type: "deleteHistoryReplayClip",
                            ...scope,
                            clip_id: c.id,
                            expected: rows!.revision,
                          }),
                        draft.id === c.id,
                      )
                    }
                  >
                    确认删除片段
                  </button>
                  <button disabled={busy} onClick={() => setRemove(null)}>
                    取消删除
                  </button>
                </>
              ) : (
                <button disabled={busy} onClick={() => setRemove(c.id)}>
                  删除片段
                </button>
              )}
            </div>
          </li>
        ))}
      </ul>
      {filtered.length > limit && (
        <button onClick={() => setLimit(limit + 20)}>再显示 20 个片段</button>
      )}
    </details>
  );
}
