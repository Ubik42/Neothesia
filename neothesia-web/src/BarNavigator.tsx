import { useEffect, useRef, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Minus,
  Plus,
  SkipBack,
  SkipForward,
} from "lucide-react";
import type { LoadedSong, Snapshot, EngineCommand } from "./api";
export function BarNavigator({
  song,
  state,
  start,
  end,
  disabled,
  recital,
  select,
  command,
}: {
  song: LoadedSong;
  state: Snapshot;
  start: number;
  end: number;
  disabled: boolean;
  recital: boolean;
  select: (a: number, b: number) => void;
  command: (c: EngineCommand) => void;
}) {
  const [visible, setVisible] = useState(16),
    [first, setFirst] = useState(1),
    [follow, setFollow] = useState(true),
    [draft, setDraft] = useState<{ a: number; b: number } | null>(null),
    [measure, setMeasure] = useState(String(state.measure)),
    [beat, setBeat] = useState("1");
  const root = useRef<HTMLDivElement>(null),
    drag = useRef<{ a: number; b: number; moved: boolean } | null>(null),
    anchor = useRef<number | null>(null);
  useEffect(() => {
    setFirst(1);
    setDraft(null);
    anchor.current = null;
  }, [song.contentId, song.scoreRevision]);
  useEffect(() => {
    setMeasure(String(state.measure));
    setBeat(String(Math.floor(state.beat)));
    if (follow)
      setFirst(Math.floor((state.measure - 1) / visible) * visible + 1);
  }, [state.measure, Math.floor(state.beat), visible, follow]);
  useEffect(() => setDraft(null), [start, end]);
  const bars = song.measures.slice(first - 1, first - 1 + visible),
    selection = draft ?? { a: start, b: end },
    low = Math.min(selection.a, selection.b),
    high = Math.max(selection.a, selection.b);
  const choose = (a: number, b: number) => {
    anchor.current = a;
    setDraft({ a, b });
    select(Math.min(a, b), Math.max(a, b));
  };
  const jump = () =>
    command({ type: "seekBeat", measure: Number(measure), beat: Number(beat) });
  return (
    <section className="bar-navigator" aria-label="小节与拍导航">
      <div className="bar-tools">
        <button
          aria-label="上一组小节"
          disabled={first === 1}
          onClick={() => {
            setFollow(false);
            setFirst(Math.max(1, first - visible));
          }}
        >
          <ChevronLeft size={13} />
        </button>
        <span>
          {first}–{bars.at(-1)?.number} / {song.measures.length}
        </span>
        <button
          aria-label="下一组小节"
          disabled={first + visible > song.measures.length}
          onClick={() => {
            setFollow(false);
            setFirst(Math.min(song.measures.length, first + visible));
          }}
        >
          <ChevronRight size={13} />
        </button>
        <button
          aria-label="放大小节"
          disabled={visible <= 4}
          onClick={() => setVisible(Math.max(4, visible / 2))}
        >
          <Plus size={13} />
        </button>
        <button
          aria-label="缩小小节"
          disabled={visible >= 64}
          onClick={() => setVisible(Math.min(64, visible * 2))}
        >
          <Minus size={13} />
        </button>
        <label>
          <input
            type="checkbox"
            checked={follow}
            onChange={(e) => setFollow(e.target.checked)}
          />
          跟随
        </label>
        <span className="bar-selection">
          {low === high ? `选中第 ${low} 小节` : `选中 ${low}–${high} 小节`}
        </span>
        <button
          disabled={disabled || recital}
          onClick={() =>
            command({
              type: "measureLoop",
              start: low,
              end: high,
              enabled: true,
            })
          }
        >
          循环选段
        </button>
      </div>
      <div
        className="bar-strip"
        ref={root}
        onPointerMove={(e) => {
          if (!drag.current) return;
          const element = document
            .elementFromPoint(e.clientX, e.clientY)
            ?.closest<HTMLElement>("[data-bar]");
          if (!element || !root.current?.contains(element)) return;
          const b = Number(element.dataset.bar);
          drag.current.b = b;
          drag.current.moved ||= b !== drag.current.a;
          setDraft({ a: drag.current.a, b });
        }}
        onPointerUp={(e) => {
          if (!drag.current) return;
          const d = drag.current;
          drag.current = null;
          if (e.currentTarget.hasPointerCapture(e.pointerId))
            e.currentTarget.releasePointerCapture(e.pointerId);
          if (d.moved) choose(d.a, d.b);
          else if (e.shiftKey && anchor.current !== null)
            choose(anchor.current, d.b);
          else {
            anchor.current = d.a;
            setDraft(null);
            command({ type: "seekMeasure", measure: d.a });
          }
        }}
        onPointerCancel={() => {
          drag.current = null;
          setDraft(null);
        }}
        onKeyDown={(e) => {
          const current = Number(
            (e.target as HTMLElement)
              .closest("[data-bar]")
              ?.getAttribute("data-bar"),
          );
          if (
            !current ||
            !["ArrowLeft", "ArrowRight"].includes(e.key) ||
            disabled ||
            recital
          )
            return;
          e.preventDefault();
          const next = Math.max(
            1,
            Math.min(
              song.measures.length,
              current + (e.key === "ArrowLeft" ? -1 : 1),
            ),
          );
          if (e.shiftKey) choose(anchor.current ?? current, next);
          else {
            anchor.current = next;
            command({ type: "seekMeasure", measure: next });
          }
          setFirst(Math.floor((next - 1) / visible) * visible + 1);
          requestAnimationFrame(() =>
            root.current
              ?.querySelector<HTMLButtonElement>(`[data-bar="${next}"]`)
              ?.focus(),
          );
        }}
      >
        {bars.map((bar) => {
          const group =
            bar.denominator === 8 &&
            bar.numerator >= 6 &&
            bar.numerator % 3 === 0
              ? 3
              : 1;
          const tickStep = ((song.ppq * 4) / bar.denominator) * group;
          const beats = Math.ceil((bar.endTick - bar.startTick) / tickStep);
          const fraction = Math.max(
            0,
            Math.min(
              1,
              (state.tick - bar.startTick) / (bar.endTick - bar.startTick),
            ),
          );
          return (
            <button
              key={bar.number}
              data-bar={bar.number}
              aria-label={`第 ${bar.number} 小节，${bar.numerator}/${bar.denominator}${bar.partial ? "，不完整小节" : ""}`}
              aria-current={
                state.measure === bar.number ? "location" : undefined
              }
              aria-pressed={bar.number >= low && bar.number <= high}
              disabled={
                disabled ||
                recital ||
                (!!state.passage &&
                  (bar.start < state.passage.start ||
                    bar.start >= state.passage.end))
              }
              className={`bar-cell ${bar.number >= low && bar.number <= high ? "in-range" : ""} ${state.measure === bar.number ? "current" : ""}`}
              onPointerDown={(e) => {
                if (e.button !== 0) return;
                e.preventDefault();
                e.currentTarget.focus();
                setFollow(false);
                drag.current = { a: bar.number, b: bar.number, moved: false };
                root.current?.setPointerCapture(e.pointerId);
              }}
              onClick={(e) => {
                if (e.detail === 0) {
                  if (e.shiftKey && anchor.current !== null)
                    choose(anchor.current, bar.number);
                  else {
                    anchor.current = bar.number;
                    command({ type: "seekMeasure", measure: bar.number });
                  }
                }
              }}
              title={`${bar.numerator}/${bar.denominator} · ${bar.partial ? "弱起或变拍截断 · " : ""}拖动选段，Shift 点击扩展`}
            >
              <span>{bar.number}</span>
              <div className="bar-beats">
                {Array.from({ length: beats }, (_, i) => (
                  <i
                    key={i}
                    style={{
                      flex:
                        Math.min(
                          tickStep,
                          bar.endTick - bar.startTick - i * tickStep,
                        ) / tickStep,
                    }}
                  />
                ))}
              </div>
              <small>
                {bar.partial
                  ? "弱起/短节"
                  : bar.numerator + "/" + bar.denominator}
              </small>
              {state.measure === bar.number && (
                <b
                  className="bar-cursor"
                  style={{ left: `${fraction * 100}%` }}
                />
              )}
            </button>
          );
        })}
      </div>
      <div className="beat-tools">
        <button
          aria-label="上一拍"
          disabled={
            disabled ||
            recital ||
            state.position <= Math.max(0, state.passage?.start ?? 0)
          }
          onClick={() => command({ type: "stepBeat", direction: -1 })}
        >
          <SkipBack size={12} />
        </button>
        <button
          aria-label="下一拍"
          disabled={disabled || recital}
          onClick={() => command({ type: "stepBeat", direction: 1 })}
        >
          <SkipForward size={12} />
        </button>
        <label>
          小节
          <input
            aria-label="跳转小节"
            type="number"
            min={1}
            max={song.measures.length}
            value={measure}
            disabled={disabled || recital}
            onChange={(e) => setMeasure(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") jump();
            }}
          />
        </label>
        <label>
          拍
          <input
            aria-label="跳转拍位"
            type="number"
            min={1}
            step={0.5}
            value={beat}
            disabled={disabled || recital}
            onChange={(e) => setBeat(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") jump();
            }}
          />
        </label>
        <button disabled={disabled || recital} onClick={jump}>
          定位
        </button>
        <span>
          {recital
            ? "完整演奏从曲首开始；选段请切换练习模式"
            : "拖动或 Shift 点击选择小节；方向键移动，Shift＋方向键扩展"}
        </span>
      </div>
    </section>
  );
}
