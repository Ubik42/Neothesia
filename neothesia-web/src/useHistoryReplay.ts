import { useEffect, useRef, useState } from "react";
import { api } from "./api";
type ReplayState = {
  id: string | null;
  active: boolean;
  paused: boolean;
  position: number;
  end: number;
  rate: number;
  loop: { start: number; end: number } | null;
  loopRounds: number;
};
export function useHistoryReplay(
  contentId: string,
  id: string,
  enabled = true,
  onPosition?: (position: number | null) => void,
  onActivity?: (active: boolean) => void,
) {
  const [phase, setPhase] = useState<
      "idle" | "running" | "paused" | "finished"
    >("idle"),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [position, setPosition] = useState(0),
    [end, setEnd] = useState(0),
    [loop, setLoop] = useState<{ start: number; end: number } | null>(null),
    [loopRounds, setLoopRounds] = useState(0);
  const owner = useRef<string | null>(null),
    epoch = useRef(0),
    alive = useRef(true),
    working = useRef(false),
    callbacks = useRef({ onPosition, onActivity });
  callbacks.current = { onPosition, onActivity };
  const stop = async () => {
    epoch.current++;
    const own = owner.current;
    owner.current = null;
    callbacks.current.onPosition?.(null);
    callbacks.current.onActivity?.(false);
    if (alive.current) {
      setPhase("idle");
      setLoop(null);
      setLoopRounds(0);
    }
    if (own)
      await api.command({ type: "stopFingerDemo", id: own }).catch(() => {});
  };
  const apply = (v: ReplayState) => {
    setPosition(v.position);
    setEnd(v.end);
    setLoop(v.loop ?? null);
    setLoopRounds(v.loopRounds ?? 0);
    setPhase(v.active ? "running" : v.paused ? "paused" : "finished");
    callbacks.current.onPosition?.(v.position);
    callbacks.current.onActivity?.(v.active || v.paused);
  };
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      void stop();
    };
  }, []);
  useEffect(() => {
    void stop();
    setPosition(0);
    setEnd(0);
    setError("");
  }, [contentId, id]);
  useEffect(() => {
    if (!enabled) void stop();
  }, [enabled]);
  useEffect(() => {
    if (phase === "idle") return;
    let polling = false;
    const tick = async () => {
      if (polling || working.current) return;
      polling = true;
      const own = owner.current,
        revision = epoch.current;
      try {
        const v = await api.command<ReplayState>({ type: "fingerDemoState" });
        if (
          !alive.current ||
          own !== owner.current ||
          revision !== epoch.current
        )
          return;
        if (v.id !== own) {
          void stop();
          return;
        }
        apply(v);
      } catch (e) {
        if (alive.current) setError(String(e));
        void stop();
      } finally {
        polling = false;
      }
    };
    const timer = setInterval(() => void tick(), 80),
      hide = () => {
        if (document.hidden) void stop();
      };
    document.addEventListener("visibilitychange", hide);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", hide);
    };
  }, [phase]);
  async function play(source: string, rate: number) {
    if (working.current || !enabled) return;
    working.current = true;
    setBusy(true);
    setError("");
    await stop();
    const request = epoch.current;
    try {
      const v = await api.command<ReplayState>({
        type: "startHistoryReplay",
        content_id: contentId,
        id,
        source,
        rate,
      });
      if (!alive.current || epoch.current !== request) {
        if (v.id) await api.command({ type: "stopFingerDemo", id: v.id });
        return;
      }
      owner.current = v.id;
      apply(v);
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function control(
    action: string,
    value?: number,
    extra?: { end: number; enabled: boolean },
  ) {
    if (working.current || !owner.current || !enabled) return;
    working.current = true;
    setBusy(true);
    setError("");
    const own = owner.current,
      request = ++epoch.current;
    try {
      const v = await api.command<ReplayState>({
        type: action === "loop" ? "historyReplayLoop" : "historyReplayControl",
        id: own,
        ...(action === "loop" ? { start: value, ...extra } : { action }),
        ...(action === "rate"
          ? { rate: value }
          : action === "seek"
            ? { position: value }
            : {}),
      });
      if (alive.current && epoch.current === request && owner.current === own)
        apply(v);
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  }
  return {
    contentId,
    historyId: id,
    phase,
    busy,
    error,
    position,
    end,
    loop,
    loopRounds,
    play,
    stop,
    control,
  };
}
export type HistoryReplay = ReturnType<typeof useHistoryReplay>;
