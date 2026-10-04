import { useEffect, useState } from "react";
import type { EngineCommand, LoadedSong, TrackSound } from "./api";
export function TrackSoundControl({
  track,
  sound,
  presets,
  disabled,
  builtin,
  action,
}: {
  track: LoadedSong["tracks"][number];
  sound?: TrackSound;
  presets: LoadedSong["soundPresets"];
  disabled: boolean;
  builtin: boolean;
  action: (c: EngineCommand) => void;
}) {
  const [volume, setVolume] = useState(sound?.volume ?? 100);
  const [pan, setPan] = useState(sound?.pan?.toString() ?? "");
  const [program, setProgram] = useState(sound?.program?.toString() ?? "");
  useEffect(() => {
    setVolume(sound?.volume ?? 100);
    setPan(sound?.pan?.toString() ?? "");
    setProgram(sound?.program?.toString() ?? "");
  }, [sound?.volume, sound?.pan, sound?.program]);
  const dirty =
    volume !== (sound?.volume ?? 100) ||
    pan !== (sound?.pan?.toString() ?? "") ||
    program !== (sound?.program?.toString() ?? "");
  return (
    <details className="track-sound">
      <summary>
        {sound
          ? `声音 · ${sound.volume}%${sound.program != null ? " · 自选音色" : ""}`
          : "声音设置"}
      </summary>
      <div className="track-sound-fields">
        <label>
          伴奏音量 <span>{volume}%</span>
          <input
            aria-label={`${track.name}伴奏音量`}
            type="range"
            min="0"
            max="100"
            step="1"
            value={volume}
            disabled={disabled}
            onChange={(e) => setVolume(Number(e.target.value))}
          />
        </label>
        <label>
          声像
          <select
            aria-label={`${track.name}声像`}
            value={pan}
            disabled={disabled}
            onChange={(e) => setPan(e.target.value)}
          >
            <option value="">跟随原曲</option>
            <option value="0">最左</option>
            <option value="32">偏左</option>
            <option value="64">中央</option>
            <option value="96">偏右</option>
            <option value="127">最右</option>
            {pan && !["0", "32", "64", "96", "127"].includes(pan) && (
              <option value={pan}>
                {Number(pan) < 64 ? "偏左" : "偏右"} · {pan}
              </option>
            )}
          </select>
        </label>
        <label>
          内置钢琴音色
          <select
            aria-label={`${track.name}音色`}
            value={program}
            disabled={disabled || !builtin}
            onChange={(e) => setProgram(e.target.value)}
          >
            <option value="">跟随原曲</option>
            {presets?.map((p) => (
              <option key={p.program} value={p.program}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <div className="track-sound-actions">
          <button
            disabled={disabled || !dirty}
            onClick={() =>
              action({
                type: "trackSound",
                id: track.id,
                sound: {
                  volume,
                  pan: pan === "" ? null : Number(pan),
                  program: program === "" ? null : Number(program),
                },
              })
            }
          >
            应用并保存
          </button>
          <button
            disabled={disabled || (!sound && !dirty)}
            onClick={() => {
              if (sound)
                action({ type: "trackSound", id: track.id, sound: null });
              else {
                setVolume(100);
                setPan("");
                setProgram("");
              }
            }}
          >
            恢复原曲
          </button>
        </div>
        <p className="parameter-help">
          用于此音轨的伴奏和试听，保留音符力度。应用会暂停并重置当前轮次，按曲目自动保存；键盘输入单独使用通道
          1。
          {!builtin
            ? "外部音源的音量、声像取决于设备支持；自选钢琴音色仅用于内置音源。"
            : ""}
        </p>
      </div>
    </details>
  );
}
