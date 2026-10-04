import { FingerDemoKeyboard } from "./FingerDemoKeyboard";
import { useEffect, useRef, useState } from "react";
import { api, pitchName } from "./api";
type DemoState = {
  id: string | null;
  active: boolean;
  paused?: boolean;
  position?: number;
  start?: number;
  end?: number;
  rate?: number;
  fingerNotes?: {
    track: number;
    index: number;
    pitch: number;
    part: string;
    finger: number | null;
    changed?: boolean;
  }[];
};
type DemoResult = {
  fingerprint: string;
  request: Record<string, unknown>;
  context: { onset: number; end: number; target: boolean }[];
};

export function FingerDemoControls({
  result,
  plan,
  time,
  active,
  move,
  disabled,
  pair,
  measures,
  pitches,
}: {
  result: DemoResult;
  plan: unknown;
  time: (value: number | null) => void;
  active: (value: boolean) => void;
  move: (index: number) => void;
  disabled: boolean;
  pair?: { parts: unknown[]; proof: string; acknowledged: boolean };
  measures?: { number: number; start: number; end: number }[];
  pitches?: number[];
}) {
  const [rate, setRate] = useState(0.5),
    [playing, setPlaying] = useState(false),
    [starting, setStarting] = useState(false),
    [error, setError] = useState(""),
    [status, setStatus] = useState("");
  const [notes, setNotes] = useState<NonNullable<DemoState["fingerNotes"]>>([]);
  const [live, setLive] = useState<DemoState | null>(null),
    [controlling, setControlling] = useState(false);
  const token = useRef<string | null>(null),
    alive = useRef(true),
    querying = useRef(false),
    epoch = useRef(0),
    launch = useRef(0);
  const apply = (data: DemoState) => {
    if (!alive.current) return;
    if (data.id !== token.current) {
      setNotes([]);
      setLive(null);
      setPlaying(false);
      active(false);
      setStatus("示范已停止");
      token.current = null;
      time(null);
      return;
    }
    setLive(data);
    setNotes(data.active || data.paused ? (data.fingerNotes ?? []) : []);
    const position = data.position!;
    time(position);
    let index = 0;
    for (let i = 0; i < result.context.length; i++)
      if (result.context[i].onset <= position) index = i;
    move(index);
    setPlaying(data.active);
    active(data.active);
    setStatus(
      data.active ? "正在示范" : data.paused ? "示范已暂停" : "示范完成",
    );
    if (!data.active && !data.paused && !pair) {
      const id = token.current;
      token.current = null;
      if (id) void api.command({ type: "stopFingerDemo", id }).catch(() => {});
    }
  };
  useEffect(() => {
    const ticket = ++epoch.current;
    alive.current = true;
    setStarting(false);
    setNotes([]);
    setLive(null);
    setControlling(false);
    time(null);
    active(false);
    setPlaying(false);
    setStatus("");
    setError("");
    const poll = async () => {
      if (!token.current || querying.current) return;
      querying.current = true;
      const queriedId = token.current;
      try {
        const data = await api.command<DemoState>({ type: "fingerDemoState" });
        if (epoch.current === ticket && token.current === queriedId)
          apply(data);
      } catch (e) {
        if (alive.current) {
          setError(String(e));
          setPlaying(false);
          active(false);
        }
      } finally {
        querying.current = false;
      }
    };
    const timer = setInterval(() => void poll(), 80);
    const hide = () => {
      if (document.hidden && token.current) {
        const id = token.current;
        token.current = null;
        void api.command({ type: "stopFingerDemo", id }).catch(() => {});
        setNotes([]);
        setLive(null);
        setPlaying(false);
        active(false);
        time(null);
        setStatus("示范已停止");
      }
    };
    document.addEventListener("visibilitychange", hide);
    return () => {
      alive.current = false;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", hide);
      const id = token.current;
      token.current = null;
      if (id) void api.command({ type: "stopFingerDemo", id }).catch(() => {});
    };
  }, [result.fingerprint, plan, disabled, pair?.acknowledged]);
  const start = async () => {
    if (starting) return;
    const ticket = epoch.current,
      launchTicket = ++launch.current;
    setStarting(true);
    setError("");
    try {
      const data = await api.command<DemoState>(
        pair
          ? {
              type: "startPairFingerDemo",
              ...pair,
              rate,
            }
          : {
              type: "startFingerDemo",
              request: result.request,
              fingerprint: result.fingerprint,
              rate,
            },
      );
      if (
        document.hidden ||
        !alive.current ||
        epoch.current !== ticket ||
        launch.current !== launchTicket
      ) {
        if (data.id) await api.command({ type: "stopFingerDemo", id: data.id });
        return;
      }
      token.current = data.id;
      apply(data);
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      if (
        alive.current &&
        epoch.current === ticket &&
        launch.current === launchTicket
      )
        setStarting(false);
    }
  };
  const stop = async () => {
    launch.current++;
    setStarting(false);
    const id = token.current;
    token.current = null;
    setNotes([]);
    setPlaying(false);
    active(false);
    time(null);
    setStatus("示范已停止");
    setLive(null);
    if (id)
      try {
        await api.command({ type: "stopFingerDemo", id });
      } catch (e) {
        if (alive.current) setError(String(e));
      }
  };
  const control = async (action: string, position?: number) => {
    const id = token.current,
      ticket = epoch.current;
    if (!id || controlling) return;
    setControlling(true);
    setError("");
    try {
      const data = await api.command<DemoState>({
        type: "fingerDemoControl",
        id,
        action,
        position: position ?? null,
      });
      if (alive.current && ticket === epoch.current && token.current === id)
        apply(data);
    } catch (e) {
      if (alive.current && ticket === epoch.current)
        setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (alive.current && ticket === epoch.current) setControlling(false);
    }
  };
  const currentMeasure = measures?.find(
    (m) => m.start <= (live?.position ?? 0) && (live?.position ?? 0) < m.end,
  );
  return (
    <div
      className="finger-demo-controls"
      role={pair ? "region" : undefined}
      aria-label={pair ? "双手同步示范" : undefined}
    >
      <label>
        示范速度{" "}
        <select
          aria-label={pair ? "双手示范速度" : "指法示范速度"}
          value={rate}
          disabled={playing || starting}
          onChange={(e) => setRate(Number(e.target.value))}
        >
          {[0.25, 0.5, 0.75, 1].map((value) => (
            <option key={value} value={value}>
              {value * 100}%
            </option>
          ))}
        </select>
      </label>
      <button
        disabled={disabled || starting || playing}
        onClick={() => void start()}
      >
        {starting
          ? "正在准备示范"
          : pair
            ? "示范双手审阅结果"
            : "示范整个建议选段"}
      </button>
      <button
        disabled={!token.current && !starting}
        onClick={() => void stop()}
      >
        停止指法示范
      </button>
      {pair && (
        <button
          disabled={
            !token.current ||
            controlling ||
            starting ||
            (live?.position ?? 0) >= (live?.end ?? 0)
          }
          onClick={() => void control(playing ? "pause" : "resume")}
        >
          {playing ? "暂停示范" : "继续示范"}
        </button>
      )}
      <span role="status">{status}</span>
      {pair && live && (
        <div className="finger-demo-navigation">
          <label>
            示范位置 ·{" "}
            {currentMeasure ? `第 ${currentMeasure.number} 小节` : "选段结束"}
            <input
              type="range"
              aria-label="双手示范位置"
              min={live.start}
              max={live.end}
              step={0.01}
              value={live.position}
              disabled={controlling || starting}
              onChange={(e) => void control("seek", Number(e.target.value))}
            />
          </label>
          <label>
            定位小节
            <select
              aria-label="双手示范定位小节"
              value={currentMeasure?.number ?? ""}
              disabled={controlling || starting}
              onChange={(e) => {
                const m = measures?.find(
                  (m) => m.number === Number(e.target.value),
                );
                if (m) void control("seek", Math.max(m.start, live.start ?? 0));
              }}
            >
              <option value="" disabled>
                选段结束
              </option>
              {measures
                ?.filter(
                  (m) => m.start < (live.end ?? 0) && m.end > (live.start ?? 0),
                )
                .map((m) => (
                  <option key={m.number} value={m.number}>
                    第 {m.number} 小节
                  </option>
                ))}
            </select>
          </label>
        </div>
      )}
      {pair && <FingerDemoKeyboard notes={notes} pitches={pitches ?? []} />}

      {pair && (
        <div className="finger-pair-live" aria-live="off">
          {["left", "right"].map((hand) => (
            <div key={hand}>
              <strong>{hand === "left" ? "左手" : "右手"}</strong>
              {notes.filter((n) => n.part === hand).length ? (
                notes
                  .filter((n) => n.part === hand)
                  .map((n) => (
                    <span
                      key={`${n.track}:${n.index}`}
                      className="finger-live-note"
                    >
                      {pitchName(n.pitch)} ·{" "}
                      {n.finger ? `${n.finger} 指` : "未标记"}
                      {n.changed ? " · 已换指" : ""}
                    </span>
                  ))
              ) : (
                <span>{playing ? "休止" : "等待示范"}</span>
              )}
            </div>
          ))}
        </div>
      )}
      <small>
        {pair
          ? "按共同小节范围示范双手、起点持音和已有换指；采用已勾选的指法及保留标记，不计成绩、不保存指法。"
          : "按原节奏示范选段及起点持音，不计成绩、不保存指法。"}
        切换方案、开始练习、按下琴键或关闭窗口会停止。
      </small>
      {error && (
        <p role="alert" className="dialog-error">
          {error}
        </p>
      )}
    </div>
  );
}
