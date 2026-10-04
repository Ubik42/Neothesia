import { useEffect, useState } from "react";
import {
  api,
  type LoadedSong,
  type Snapshot,
  type SpeedLadderPlan,
  type LadderPreset,
  type LadderRun,
  type EngineCommand,
} from "./api";
export const ladderSpeeds = (plan: SpeedLadderPlan) => {
  if (plan.stepPercent < 1 || plan.startPercent > plan.targetPercent) return [];
  const speeds = [plan.startPercent];
  while (speeds[speeds.length - 1] < plan.targetPercent && speeds.length < 200)
    speeds.push(
      Math.min(
        plan.targetPercent,
        speeds[speeds.length - 1] + plan.stepPercent,
      ),
    );
  return speeds;
};
export function LadderStatus({
  state,
  action,
  manage,
}: {
  state: Snapshot;
  action: (c: EngineCommand) => void;
  manage: () => void;
}) {
  const run = state.ladder;
  if (!run) return null;
  const speeds = ladderSpeeds(run.preset.plan);
  return (
    <div className="ladder-status" aria-live="polite">
      <div>
        <strong>{run.preset.name}</strong>
        <span>
          第 {run.progress.stage + 1}/{speeds.length} 级 ·{" "}
          {speeds[run.progress.stage]}% → {run.preset.plan.targetPercent}% ·
          连续达标 {run.progress.successStreak}/{run.preset.plan.passesRequired}{" "}
          轮 · 已练 {run.progress.totalRounds} 轮
        </span>
        <span>
          正确率 ≥{run.preset.plan.accuracyPercent}%
          {run.preset.plan.onTimePercent !== null &&
            ` · 准时率 ≥${run.preset.plan.onTimePercent}%`}
        </span>
      </div>
      <span className="ladder-outcome">
        {run.progress.completed
          ? "目标速度已达标"
          : run.progress.limited
            ? "本次轮数已到上限"
            : run.progress.lastResult ||
              (state.status === "ready"
                ? "已应用，按开始练习"
                : "等待本轮结果")}
      </span>
      <button onClick={manage}>查看方案</button>
      {run.active && (
        <button onClick={() => action({ type: "stopLadder" })}>结束阶梯</button>
      )}
    </div>
  );
}
export function LadderDialog({
  song,
  state,
  start,
  end,
  close,
  changed,
}: {
  song: LoadedSong;
  state: Snapshot;
  start: number;
  end: number;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [presets, setPresets] = useState<LadderPreset[]>([]),
    [runs, setRuns] = useState<LadderRun[]>([]),
    [id, setId] = useState<string | null>(null);
  const [name, setName] = useState(""),
    [first, setFirst] = useState(start),
    [last, setLast] = useState(end),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [plan, setPlan] = useState<SpeedLadderPlan>({
    startPercent: Math.max(25, Math.min(100, Math.round(state.speed * 100))),
    targetPercent: 100,
    stepPercent: 5,
    passesRequired: 2,
    accuracyPercent: 90,
    onTimePercent: null,
    failuresBeforeStepBack: 0,
    attemptLimit: 50,
  });
  const active = Boolean(state.ladder?.active),
    speeds = ladderSpeeds(plan),
    locked = busy || active;
  const reload = async () => {
    const data = await api.command<{
      presets: LadderPreset[];
      runs: LadderRun[];
    }>({ type: "ladderPresets" });
    setPresets(data.presets);
    setRuns(data.runs);
  };
  useEffect(() => {
    void reload().catch((e) => setError(String(e.message)));
  }, [song.contentId]);
  useEffect(() => {
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !busy) close();
    };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [busy, close]);
  const work = async (job: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await job();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };
  const patch = (value: Partial<SpeedLadderPlan>) =>
    setPlan((p) => ({ ...p, ...value }));
  const select = (p: LadderPreset) => {
    setId(p.id);
    setName(p.name);
    setFirst(p.start);
    setLast(p.end);
    setPlan(p.plan);
    setError("");
  };
  const apply = (presetId: string, resume: boolean) =>
    void work(async () => {
      await api.command({ type: "startLadder", id: presetId, resume });
      await changed();
      close();
    });
  const saved = presets.find((p) => p.id === id),
    checkpoint =
      state.ladder?.preset.id === id
        ? state.ladder
        : runs.find((r) => r.preset.id === id);
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog ladder-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ladder-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <h2 id="ladder-title">速度阶梯</h2>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭速度阶梯"
            onClick={close}
          >
            关闭
          </button>
        </div>
        {error && (
          <p role="alert" className="ladder-error">
            {error}
          </p>
        )}
        <div className="ladder-layout">
          <aside className="ladder-library">
            <div className="ladder-list-heading">
              <strong>本曲方案</strong>
              <button
                disabled={locked}
                onClick={() => {
                  setId(null);
                  setName("");
                  setFirst(start);
                  setLast(end);
                }}
              >
                新建
              </button>
            </div>
            {presets.length === 0 ? (
              <p className="parameter-help">
                还没有方案。设置小节范围和达标标准后保存。
              </p>
            ) : (
              presets.map((p) => {
                const run = runs.find((r) => r.preset.id === p.id);
                return (
                  <button
                    className={`ladder-preset ${id === p.id ? "selected" : ""}`}
                    key={p.id}
                    disabled={busy}
                    onClick={() => select(p)}
                  >
                    <strong>{p.name}</strong>
                    <span>
                      第 {p.start}–{p.end} 小节 · {p.plan.startPercent}% →{" "}
                      {p.plan.targetPercent}%
                    </span>
                    <small>
                      {run
                        ? run.progress.completed
                          ? "已完成"
                          : `已练 ${run.progress.totalRounds} 轮 · 停在第 ${run.progress.stage + 1} 级`
                        : "尚未练习"}
                    </small>
                  </button>
                );
              })
            )}
            {active && (
              <button
                className="ladder-stop"
                disabled={busy}
                onClick={() =>
                  void work(async () => {
                    await api.command({ type: "stopLadder" });
                    await changed();
                    await reload();
                  })
                }
              >
                结束当前阶梯，保留进度
              </button>
            )}
          </aside>
          <div className="ladder-editor">
            <label className="ladder-name">
              方案名称
              <input
                aria-label="阶梯方案名称"
                value={name}
                maxLength={80}
                disabled={locked}
                placeholder="例如：右手第 1–4 小节提速"
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <div className="ladder-form-grid">
              <label>
                开始小节
                <input
                  aria-label="阶梯开始小节"
                  type="number"
                  min={1}
                  max={song.measures.length}
                  value={first}
                  disabled={locked}
                  onChange={(e) => setFirst(Number(e.target.value))}
                />
              </label>
              <label>
                结束小节
                <input
                  aria-label="阶梯结束小节"
                  type="number"
                  min={first}
                  max={song.measures.length}
                  value={last}
                  disabled={locked}
                  onChange={(e) => setLast(Number(e.target.value))}
                />
              </label>
              <label>
                起始速度 %
                <input
                  aria-label="阶梯起始速度"
                  type="number"
                  min={25}
                  max={200}
                  value={plan.startPercent}
                  disabled={locked}
                  onChange={(e) =>
                    patch({ startPercent: Number(e.target.value) })
                  }
                />
              </label>
              <label>
                目标速度 %
                <input
                  aria-label="阶梯目标速度"
                  type="number"
                  min={plan.startPercent}
                  max={200}
                  value={plan.targetPercent}
                  disabled={locked}
                  onChange={(e) =>
                    patch({ targetPercent: Number(e.target.value) })
                  }
                />
              </label>
              <label>
                每级增加百分点
                <input
                  aria-label="阶梯速度步长"
                  type="number"
                  min={1}
                  max={50}
                  value={plan.stepPercent}
                  disabled={locked}
                  onChange={(e) =>
                    patch({ stepPercent: Number(e.target.value) })
                  }
                />
              </label>
              <label>
                连续达标轮数
                <input
                  aria-label="阶梯连续达标轮数"
                  type="number"
                  min={1}
                  max={10}
                  value={plan.passesRequired}
                  disabled={locked}
                  onChange={(e) =>
                    patch({ passesRequired: Number(e.target.value) })
                  }
                />
              </label>
              <label>
                正确率至少 %
                <input
                  aria-label="阶梯正确率门槛"
                  type="number"
                  min={50}
                  max={100}
                  value={plan.accuracyPercent}
                  disabled={locked}
                  onChange={(e) =>
                    patch({ accuracyPercent: Number(e.target.value) })
                  }
                />
              </label>
              <label>
                本次轮数上限
                <input
                  aria-label="阶梯轮数上限"
                  type="number"
                  min={1}
                  max={500}
                  value={plan.attemptLimit}
                  disabled={locked}
                  onChange={(e) =>
                    patch({ attemptLimit: Number(e.target.value) })
                  }
                />
              </label>
            </div>
            <label className="ladder-checkbox">
              <input
                aria-label="阶梯要求准时率"
                type="checkbox"
                disabled={locked || state.mode !== "flow"}
                checked={plan.onTimePercent !== null}
                onChange={(e) =>
                  patch({ onTimePercent: e.target.checked ? 80 : null })
                }
              />
              同时要求准时率（连续模式）
              {plan.onTimePercent !== null && (
                <input
                  aria-label="阶梯准时率门槛"
                  type="number"
                  min={50}
                  max={100}
                  value={plan.onTimePercent}
                  disabled={locked}
                  onChange={(e) =>
                    patch({ onTimePercent: Number(e.target.value) })
                  }
                />
              )}
            </label>
            <label className="ladder-failure">
              未达标处理
              <select
                aria-label="阶梯未达标处理"
                disabled={locked}
                value={plan.failuresBeforeStepBack}
                onChange={(e) =>
                  patch({ failuresBeforeStepBack: Number(e.target.value) })
                }
              >
                <option value={0}>重复当前速度</option>
                {[1, 2, 3, 5].map((n) => (
                  <option key={n} value={n}>
                    连续 {n} 轮未达标，回退一级
                  </option>
                ))}
              </select>
            </label>
            <div className="ladder-levels" aria-label="阶梯速度级别">
              {speeds.map((speed, i) => (
                <span key={speed}>
                  {i + 1}. {speed}%
                </span>
              ))}
            </div>
            <p className="parameter-help">
              保存采用当前{state.mode === "wait" ? "等音" : "连续"}
              模式、声部、左右手、预备和节拍器设置。每一级连续达标{" "}
              {plan.passesRequired}{" "}
              轮才晋级，目标级也需要达标；错漏音都会降低正确率。提前保存不计入晋级。
            </p>
            {saved && (
              <p className="parameter-help">
                已存方案：{saved.settings.mode === "wait" ? "等音" : "连续"} ·{" "}
                {saved.settings.hands === "left"
                  ? "左手"
                  : saved.settings.hands === "right"
                    ? "右手"
                    : saved.settings.hands === "custom"
                      ? "自定义声部"
                      : "双手"}{" "}
                · 预备 {saved.settings.countIn}{" "}
                小节。更改范围、声部或晋级规则会清除进度；仅改名保留进度。
              </p>
            )}
            {checkpoint && (
              <p className="ladder-checkpoint">
                上次：{checkpoint.progress.lastResult || "已应用"} · 第{" "}
                {checkpoint.progress.stage + 1} 级 · 已练{" "}
                {checkpoint.progress.totalRounds} 轮
                {checkpoint.progress.lastAccuracy !== null &&
                  ` · 上轮正确率 ${Math.round(checkpoint.progress.lastAccuracy * 100)}%`}
              </p>
            )}
            <div className="ladder-actions">
              <button
                disabled={locked || !name.trim()}
                onClick={() =>
                  void work(async () => {
                    const data = await api.command<{ id: string }>({
                      type: "saveLadder",
                      id,
                      name,
                      start: first,
                      end: last,
                      plan,
                    });
                    setId(data.id);
                    await reload();
                  })
                }
              >
                保存方案
              </button>
              {id && (
                <>
                  <button disabled={locked} onClick={() => apply(id, false)}>
                    从起始速度应用
                  </button>
                  <button
                    disabled={
                      locked || !checkpoint || checkpoint.progress.completed
                    }
                    onClick={() => apply(id, true)}
                  >
                    继续保存进度
                  </button>
                  <button
                    disabled={locked}
                    onClick={() =>
                      void work(async () => {
                        await api.command({ type: "removeLadder", id });
                        setId(null);
                        setName("");
                        await reload();
                      })
                    }
                  >
                    移除方案
                  </button>
                </>
              )}
            </div>
            <p className="parameter-help">
              应用后按主界面的“开始练习”。暂停可以继续；结束阶梯后仍保留进度。关闭软件后不会自动演奏。
            </p>
          </div>
        </div>
      </section>
    </div>
  );
}
