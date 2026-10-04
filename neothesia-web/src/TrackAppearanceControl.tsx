import { useEffect, useState } from "react";
import type { LoadedSong, EngineCommand } from "./api";
export const trackColors = [
  "#6cb7f3",
  "#a9df74",
  "#f1ad67",
  "#de94dc",
  "#8dd6ca",
  "#b4a3f0",
  "#f08b9c",
  "#ced3de",
];
export function TrackAppearanceControl({
  track,
  disabled,
  action,
}: {
  track: LoadedSong["tracks"][number];
  disabled: boolean;
  action: (c: EngineCommand) => void;
}) {
  const [name, setName] = useState(track.name),
    [color, setColor] = useState(track.color ?? "");
  useEffect(() => {
    setName(track.name);
    setColor(track.color ?? "");
  }, [track.name, track.color]);
  const dirty = name.trim() !== track.name || color !== (track.color ?? "");
  const apply = () =>
    action({
      type: "trackAppearance",
      id: track.id,
      appearance: {
        name: name.trim() === track.sourceName ? null : name.trim(),
        color: color || null,
      },
    });
  return (
    <details className="track-appearance">
      <summary>名称与颜色</summary>
      <div className="track-appearance-fields">
        <label>
          音轨名称
          <input
            aria-label={`音轨 ${track.id + 1}名称`}
            maxLength={80}
            value={name}
            disabled={disabled}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && dirty && name.trim() && !disabled) {
                e.preventDefault();
                apply();
              }
            }}
          />
        </label>
        <small title={track.sourceName}>原曲：{track.sourceName}</small>
        <fieldset disabled={disabled}>
          <legend>音符颜色</legend>
          <div className="track-color-choices">
            <button
              type="button"
              aria-label={`音轨 ${track.id + 1}按手别配色`}
              aria-pressed={color === ""}
              onClick={() => setColor("")}
            >
              手别
            </button>
            {trackColors.map((c, i) => (
              <button
                type="button"
                key={c}
                aria-label={`音轨 ${track.id + 1}颜色 ${i + 1}`}
                aria-pressed={color === c}
                title={`颜色 ${i + 1}`}
                style={{ background: c }}
                onClick={() => setColor(c)}
              />
            ))}
            {color && !trackColors.includes(color) && (
              <button
                type="button"
                aria-label="当前自选颜色"
                aria-pressed
                style={{ background: color }}
              />
            )}
          </div>
        </fieldset>
        <div className="track-sound-actions">
          <button
            aria-label="保存名称与颜色"
            disabled={disabled || !dirty || !name.trim()}
            onClick={apply}
          >
            保存
          </button>
          <button
            aria-label="恢复名称与配色"
            disabled={disabled || (!track.appearanceCustom && !dirty)}
            onClick={() => {
              if (track.appearanceCustom)
                action({
                  type: "trackAppearance",
                  id: track.id,
                  appearance: null,
                });
              else {
                setName(track.sourceName ?? track.name);
                setColor("");
              }
            }}
          >
            恢复原设置
          </button>
        </div>
        <p className="parameter-help">
          颜色用于键盘提示；待弹音保留高亮。按曲目保存，不修改原文件，也不重置练习轮次。
        </p>
      </div>
    </details>
  );
}
