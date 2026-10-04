import { useEffect, useRef } from "react";
import type { LoadedSong, Snapshot } from "./api";
import {
  displayDefaults,
  displayPitch,
  noteLabel,
  handColor,
  type DisplayPreferences,
} from "./displayPreferences";

const sharp = (pitch: number) => [1, 3, 6, 8, 10].includes(pitch % 12);
function keyGeometry(pitch: number, width: number, white: number[]) {
  const w = width / white.length;
  const index = white.filter((p) => p < pitch).length;
  return sharp(pitch)
    ? { x: index * w - w * 0.31, w: w * 0.62 }
    : { x: index * w, w };
}
export function Piano({
  song,
  state,
  onNote,
  fullRange = true,
  previewBars = 4,
  onSelect,
  display = displayDefaults,
}: {
  song: LoadedSong | null;
  state: Snapshot;
  onNote: (pitch: number, active: boolean) => void;
  fullRange?: boolean;
  previewBars?: number;
  display?: DisplayPreferences;
  onSelect?: (note: LoadedSong["notes"][number]) => void;
}) {
  const pitches = Array.from(
    { length: fullRange ? 88 : 49 },
    (_, i) => (fullRange ? 21 : 36) + i,
  );
  const white = pitches.filter((p) => !sharp(p));
  const geometry = (pitch: number, width: number) =>
    keyGeometry(pitch, width, white);
  const canvas = useRef<HTMLCanvasElement>(null);
  const latest = useRef({ song, state, received: performance.now() });
  const actionNotes = useRef(new Map<string, LoadedSong["notes"][number]>());
  useEffect(() => {
    actionNotes.current = new Map(
      (song?.notes ?? []).map((n) => [`${n.track}:${n.index}`, n]),
    );
  }, [song]);
  const held = useRef(new Map<number, number>());
  const hitNotes = useRef<
    {
      x: number;
      y: number;
      w: number;
      h: number;
      note: LoadedSong["notes"][number];
    }[]
  >([]);
  useEffect(() => {
    latest.current = { song, state, received: performance.now() };
  }, [song, state]);
  useEffect(() => {
    const element = canvas.current!;
    let frame = 0;
    const draw = () => {
      const bounds = element.getBoundingClientRect(),
        width = element.clientWidth,
        height = element.clientHeight,
        dpr =
          (window.devicePixelRatio || 1) * (bounds.width / Math.max(1, width));
      if (
        element.width !== Math.round(width * dpr) ||
        element.height !== Math.round(height * dpr)
      ) {
        element.width = Math.round(width * dpr);
        element.height = Math.round(height * dpr);
      }
      const ctx = element.getContext("2d")!;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, width, height);
      hitNotes.current = [];
      const { song, state, received } = latest.current;
      const keyboardHeight = Math.min(display.keyboardHeight, height * 0.45),
        baseline = height - keyboardHeight;
      const position =
        state.position +
        (state.status === "playing" && state.required.length === 0
          ? Math.min((performance.now() - received) / 1000, 0.12) * state.speed
          : 0);
      const gradient = ctx.createLinearGradient(0, 0, 0, baseline);
      gradient.addColorStop(0, "#12171e");
      gradient.addColorStop(1, "#1c2430");
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, width, baseline);
      for (const pitch of white) {
        const { x } = geometry(pitch, width);
        ctx.strokeStyle =
          pitch % 12 === 0 ? "rgba(166,185,210,.17)" : "rgba(166,185,210,.065)";
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, baseline);
        ctx.stroke();
      }
      const tempo = song?.tempo ?? [{ tick: 0, seconds: 0, bpm: 120 }];
      const at = tempo.filter((t) => t.seconds <= position).at(-1) ?? tempo[0];
      const tick =
        at.tick +
        (((position - at.seconds) * at.bpm) / 60) * (song?.ppq ?? 480);
      const bar = song?.measures.filter((m) => m.startTick <= tick).at(-1);
      const index = song?.measures.findIndex((m) => m === bar) ?? 0;
      const end =
        song?.measures[Math.max(0, index) + previewBars]?.startTick ??
        tick + (bar ? bar.endTick - bar.startTick : 1920) * previewBars;
      const span = Math.max(1, end - (bar?.startTick ?? tick)),
        pixels = baseline / span;
      for (const m of song?.measures ?? []) {
        if (m.endTick < tick) continue;
        if (m.startTick > tick + span) break;
        const beatTicks = (song!.ppq * 4) / m.denominator;
        for (let beat = 0; beat < m.numerator; beat++) {
          const t = m.startTick + beat * beatTicks;
          if (t < tick || t >= m.endTick) continue;
          const y = baseline - (t - tick) * pixels;
          ctx.strokeStyle =
            beat === 0 ? "rgba(186,203,224,.32)" : "rgba(186,203,224,.09)";
          ctx.lineWidth =
            beat === 0
              ? 1.2
              : m.denominator === 8 &&
                  m.numerator >= 6 &&
                  m.numerator % 3 === 0 &&
                  beat % 3 === 0
                ? 1
                : 0.7;
          ctx.beginPath();
          ctx.moveTo(0, y);
          ctx.lineTo(width, y);
          ctx.stroke();
          if (beat === 0) {
            ctx.fillStyle = "#aab8cc";
            ctx.font = "11px sans-serif";
            ctx.fillText(
              `第 ${m.number} 小节${m.partial ? " · 不完整" : ""}  ${m.numerator}/${m.denominator}${m.explicit ? "" : "*"}`,
              8,
              y - 6,
            );
          }
        }
      }
      ctx.lineWidth = 1;
      if (song) {
        // Binary search bounds rendering cost for long performances.
        let lo = 0,
          hi = song.notes.length;
        while (lo < hi) {
          const mid = (lo + hi) >> 1;
          if (song.notes[mid].start < position - 15) lo = mid + 1;
          else hi = mid;
        }
        for (let i = lo; i < song.notes.length; i++) {
          const note = song.notes[i];
          if (note.tick > tick + span) break;
          if (!pitches.includes(note.pitch)) continue;
          if (note.start + note.duration < position) continue;
          const { x, w } = geometry(note.pitch, width),
            y = baseline - (note.tick - tick) * pixels,
            h = Math.max(8, (note.endTick - note.tick) * pixels);
          const required =
            state.required.includes(note.pitch) &&
            Math.abs(note.start - state.position) < 0.02;
          hitNotes.current.push({ x, y: y - h, w, h, note });
          ctx.fillStyle = required
            ? "#efd484"
            : display.palette === "accessible"
              ? handColor(note.part, display)
              : (song.tracks.find((t) => t.id === note.track)?.color ??
                handColor(note.part, display));
          ctx.globalAlpha = required ? 1 : 0.88;
          ctx.beginPath();
          ctx.roundRect(x + 1, y - h, Math.max(2, w - 2), h, 3);
          ctx.fill();
          ctx.globalAlpha = 1;
          const actions = song.fingerActions
            ?.filter(
              (a) =>
                a.valid &&
                Math.abs(a.validationRate - state.speed) < 0.0001 &&
                a.track === note.track &&
                a.index === note.index &&
                a.at <= position,
            )
            .sort((a, b) => a.at - b.at);
          const label = noteLabel(
            note.pitch,
            actions?.at(-1)?.to ?? note.finger,
            display,
          );
          if (label && w > 10 && h > 16) {
            const fill = required
              ? "#efd484"
              : display.palette === "accessible"
                ? handColor(note.part, display)
                : (song.tracks.find((t) => t.id === note.track)?.color ??
                  handColor(note.part, display));
            const luminance = fill
              ? parseInt(fill.slice(1, 3), 16) * 0.299 +
                parseInt(fill.slice(3, 5), 16) * 0.587 +
                parseInt(fill.slice(5, 7), 16) * 0.114
              : 200;
            ctx.fillStyle = luminance < 145 ? "#ffffff" : "#18362e";
            ctx.font = `${display.fontSize}px sans-serif`;
            const size = Math.min(
              display.fontSize,
              (display.fontSize * (w - 4)) /
                Math.max(1, ctx.measureText(label).width),
            );
            if (size < 8) continue;
            ctx.font = `${size}px sans-serif`;
            ctx.textAlign = "center";
            ctx.fillText(label, x + w / 2, y - 8);
          }
        }
      }
      ctx.textAlign = "left";
      ctx.fillStyle = "#a9df74";
      ctx.fillRect(0, baseline - 2, width, 2);
      for (const pitch of white) {
        const { x, w } = geometry(pitch, width);
        ctx.fillStyle = state.pressed.includes(pitch)
          ? "#a9df74"
          : state.required.includes(pitch)
            ? "#f2d694"
            : "#e1e4ea";
        ctx.fillRect(x + 1, baseline, w - 2, keyboardHeight - 2);
        if (
          display.keyLabels !== "none" &&
          (display.keyLabels !== "c" || pitch % 12 === 0)
        ) {
          ctx.fillStyle = "#53635e";
          ctx.font = `${display.fontSize}px sans-serif`;
          const label = displayPitch(pitch, display);
          const size = Math.min(
            display.fontSize,
            (display.fontSize * (w - 3)) /
              Math.max(1, ctx.measureText(label).width),
          );
          if (size < 8) continue;
          ctx.font = `${size}px sans-serif`;
          ctx.textAlign = "center";
          ctx.fillText(label, x + w / 2, height - 14);
        }
      }
      for (const pitch of pitches.filter(sharp)) {
        const { x, w } = geometry(pitch, width);
        ctx.fillStyle = state.pressed.includes(pitch)
          ? "#579b89"
          : state.required.includes(pitch)
            ? "#b68e43"
            : "#11151c";
        ctx.beginPath();
        ctx.roundRect(x, baseline, w, keyboardHeight * 0.62, [0, 0, 3, 3]);
        ctx.fill();
        if (display.keyLabels === "all") {
          const label = displayPitch(pitch, display);
          ctx.font = `${display.fontSize}px sans-serif`;
          const size = Math.min(
            display.fontSize,
            (display.fontSize * (w - 3)) /
              Math.max(1, ctx.measureText(label).width),
          );
          if (size >= 8) {
            ctx.font = `${size}px sans-serif`;
            ctx.fillStyle = "#e9edf1";
            ctx.textAlign = "center";
            ctx.fillText(
              label,
              x + w / 2,
              baseline + keyboardHeight * 0.62 - 10,
            );
          }
        }
      }
      if (song && ["auto", "finger", "both"].includes(display.noteLabels)) {
        for (const a of song.fingerActions ?? []) {
          if (!a.valid || Math.abs(a.validationRate - state.speed) > 0.0001)
            continue;
          const n = actionNotes.current.get(`${a.track}:${a.index}`);
          if (
            !n ||
            position < n.start ||
            position >= n.start + n.duration ||
            !pitches.includes(n.pitch)
          )
            continue;
          const { x, w } = geometry(n.pitch, width);
          const completed = position >= a.at;
          // Show each action only until its successor on the same sustained key.
          if (
            song.fingerActions?.some(
              (b) =>
                b.track === a.track &&
                b.index === a.index &&
                b.at > a.at &&
                b.at <= position,
            )
          )
            continue;
          ctx.font = `bold ${Math.min(13, w - 4)}px sans-serif`;
          ctx.textAlign = "center";
          ctx.fillStyle = sharp(n.pitch) ? "#f2d694" : "#294432";
          ctx.fillText(
            completed ? String(a.to) : `${a.from}→${a.to}`,
            x + w / 2,
            baseline + 22,
          );
        }
      }
      ctx.textAlign = "left";
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, [fullRange, previewBars, display]);
  const release = (pointer?: number) => {
    if (pointer !== undefined) {
      const pitch = held.current.get(pointer);
      held.current.delete(pointer);
      if (
        pitch !== undefined &&
        !Array.from(held.current.values()).includes(pitch)
      )
        onNote(pitch, false);
    } else {
      new Set(held.current.values()).forEach((p) => onNote(p, false));
      held.current.clear();
    }
  };
  useEffect(
    () => () => {
      new Set(held.current.values()).forEach((p) => onNote(p, false));
      held.current.clear();
    },
    [onNote],
  );
  return (
    <canvas
      ref={canvas}
      aria-label="按小节排列的音符与可演奏键盘"
      onPointerDown={(e) => {
        const bounds = e.currentTarget.getBoundingClientRect(),
          x =
            ((e.clientX - bounds.left) * e.currentTarget.clientWidth) /
            Math.max(1, bounds.width),
          y =
            ((e.clientY - bounds.top) * e.currentTarget.clientHeight) /
            Math.max(1, bounds.height);
        const keyboardHeight = Math.min(
          display.keyboardHeight,
          e.currentTarget.clientHeight * 0.45,
        );
        if (y < e.currentTarget.clientHeight - keyboardHeight) {
          const hit = [...hitNotes.current]
            .reverse()
            .find(
              (n) => x >= n.x && x < n.x + n.w && y >= n.y && y < n.y + n.h,
            );
          if (hit) onSelect?.(hit.note);
          return;
        }
        release(e.pointerId);
        const candidates =
          y < e.currentTarget.clientHeight - keyboardHeight * 0.38
            ? [...pitches.filter(sharp), ...white]
            : white;
        const pitch = candidates.find((p) => {
          const g = geometry(p, e.currentTarget.clientWidth);
          return x >= g.x && x < g.x + g.w;
        });
        if (pitch !== undefined) {
          const already = Array.from(held.current.values()).includes(pitch);
          held.current.set(e.pointerId, pitch);
          if (!already) onNote(pitch, true);
          e.currentTarget.setPointerCapture(e.pointerId);
        }
      }}
      onPointerUp={(e) => release(e.pointerId)}
      onPointerCancel={(e) => release(e.pointerId)}
      onLostPointerCapture={(e) => release(e.pointerId)}
    />
  );
}
