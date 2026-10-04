import { pitchName } from "./api";
export type LiveFinger = {
  track: number;
  index: number;
  pitch: number;
  part: string;
  finger: number | null;
  changed?: boolean;
};
const black = (pitch: number) => [1, 3, 6, 8, 10].includes(pitch % 12);
export function FingerDemoKeyboard({
  notes,
  pitches,
}: {
  notes: LiveFinger[];
  pitches: number[];
}) {
  const low = Math.max(
      0,
      Math.floor(Math.min(...(pitches.length ? pitches : [48])) / 12) * 12,
    ),
    high = Math.min(
      127,
      Math.ceil((Math.max(...(pitches.length ? pitches : [72])) + 1) / 12) *
        12 -
        1,
    );
  const keys = Array.from({ length: high - low + 1 }, (_, i) => low + i),
    whites = keys.filter((p) => !black(p));
  const x = (p: number) =>
    black(p)
      ? whites.filter((w) => w < p).length * 28 - 9
      : whites.indexOf(p) * 28;
  return (
    <div className="finger-demo-keyboard">
      <svg
        viewBox={`0 0 ${whites.length * 28} 142`}
        style={{ minWidth: Math.max(420, whites.length * 22) }}
        role="img"
        aria-label="双手示范键盘：蓝色左手，橙色右手；数字为当前手指"
      >
        {whites.map((p) => (
          <g key={p}>
            <rect
              x={x(p)}
              y={0}
              width={28}
              height={138}
              rx={2}
              fill="#e9edf0"
              stroke="#75818e"
            />
            <text
              x={x(p) + 14}
              y={130}
              textAnchor="middle"
              fill="#52606d"
              fontSize={9}
            >
              {p % 12 === 0 ? pitchName(p) : ""}
            </text>
          </g>
        ))}
        {keys.filter(black).map((p) => (
          <rect
            key={p}
            x={x(p)}
            y={0}
            width={18}
            height={86}
            rx={2}
            fill="#151b24"
            stroke="#637183"
          />
        ))}
        {notes.map((n) => {
          const b = black(n.pitch),
            cx = x(n.pitch) + (b ? 9 : 14),
            cy = (b ? 30 : 93) + (n.part === "right" ? 22 : 0);
          return (
            <g key={`${n.track}:${n.index}`}>
              <title>
                {n.part === "left" ? "左手" : "右手"} {pitchName(n.pitch)}，
                {n.finger ? `${n.finger} 指` : "未标记"}
                {n.changed ? "，已换指" : ""}
              </title>
              <circle
                cx={cx}
                cy={cy}
                r={10}
                fill={n.part === "left" ? "#69b9ef" : "#f3ad68"}
                stroke={n.changed ? "#fff" : "#17212d"}
                strokeWidth={n.changed ? 2 : 1}
              />
              <text
                x={cx}
                y={cy + 4}
                textAnchor="middle"
                fill="#142331"
                fontSize={12}
                fontWeight={700}
              >
                {n.finger ?? "?"}
              </text>
            </g>
          );
        })}
      </svg>
      <div className="finger-demo-keyboard-legend">
        <span>● 左手</span>
        <span>● 右手</span>
        <small>数字对应手指；? 表示未标记。此图表示按键位置。</small>
      </div>
    </div>
  );
}
