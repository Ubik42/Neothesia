import { useState } from "react";
import { X } from "lucide-react";
import { api, type LoadedSong } from "./api";
export function MeterDialog({
  song,
  close,
  changed,
}: {
  song: LoadedSong;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const old = song.meterCorrection,
    original = song.measures[0];
  const [numerator, setNumerator] = useState(
      old?.numerator ?? original?.numerator ?? 4,
    ),
    [denominator, setDenominator] = useState(
      old?.denominator ?? original?.denominator ?? 4,
    );
  const [pickup, setPickup] = useState(
      old ? old.pickup_ticks / ((song.ppq * 4) / old.denominator) : 0,
    ),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const save = async (reset = false) => {
    setBusy(true);
    setError("");
    try {
      await api.command({
        type: "meter",
        value: reset
          ? null
          : {
              numerator,
              denominator,
              pickup_ticks: Math.round((pickup * song.ppq * 4) / denominator),
            },
      });
      await changed();
      close();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="modal-backdrop" onClick={close}>
      <section
        className="settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="meter-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="meter-title">修正小节网格</h2>
            <p>{song.title}</p>
          </div>
          <button autoFocus aria-label="关闭小节网格" onClick={close}>
            <X size={18} />
          </button>
        </div>
        <p className="parameter-help">
          用于缺少拍号、拍号错误或有弱起的
          MIDI。修正应用于整首曲目，音符和速度保持原样。已有变拍子曲目应保留原始网格。
        </p>
        <div className="fingering-options">
          <label>
            每小节拍数
            <input
              aria-label="网格每小节拍数"
              type="number"
              min={1}
              max={32}
              value={numerator}
              onChange={(e) => setNumerator(Number(e.target.value))}
            />
          </label>
          <label>
            单位音符
            <select
              aria-label="网格拍号分母"
              value={denominator}
              onChange={(e) => setDenominator(Number(e.target.value))}
            >
              {[1, 2, 4, 8, 16, 32].map((n) => (
                <option key={n} value={n}>
                  {n} 分音符
                </option>
              ))}
            </select>
          </label>
          <label>
            首小节弱起拍数
            <input
              aria-label="网格弱起拍数"
              type="number"
              min={0}
              max={numerator}
              step={0.25}
              value={pickup}
              onChange={(e) => setPickup(Number(e.target.value))}
            />
          </label>
        </div>
        <p className="parameter-help">
          当前：
          {old
            ? "手动修正"
            : original?.explicit
              ? "文件拍号"
              : "缺少拍号，暂按 4/4"}
          。保存后清除当前循环范围和本轮成绩；历史记录保留，并按网格与练习音符范围分别统计。
        </p>
        {error && (
          <p role="alert" className="dialog-error">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          <button disabled={busy} onClick={() => void save(true)}>
            恢复文件原始网格
          </button>
          <button onClick={close}>取消</button>
          <button
            className="primary-button"
            disabled={busy}
            onClick={() => void save()}
          >
            保存小节网格
          </button>
        </div>
      </section>
    </div>
  );
}
