import { useState } from "react";
import { pitchName } from "./api";

export type ComparedNote = {
  track: number;
  index: number;
  measure: number;
  pitch: number;
  finger: number;
  reason: string;
};
export type ComparedPlan = {
  proposals: ComparedNote[];
  crossings: number;
  shifts: number;
  wide: number;
  changed: number;
};
const key = (n: ComparedNote) => `${n.track}:${n.index}`;
export function FingerPlanComparison({
  plans,
  current,
  revision,
  disabled,
  label,
  trackName,
  apply,
  locate,
}: {
  plans: ComparedPlan[];
  current: ComparedNote[];
  revision: string;
  disabled: boolean;
  label: string;
  trackName: (track: number) => string;
  apply: (notes: ComparedNote[]) => void;
  locate: (note: ComparedNote) => void;
}) {
  const [open, setOpen] = useState(false),
    [candidate, setCandidate] = useState(0);
  const [snapshot, setSnapshot] = useState<ComparedNote[]>([]),
    [source, setSource] = useState("");
  const [chosen, setChosen] = useState<Set<string>>(new Set()),
    [measure, setMeasure] = useState("all"),
    [page, setPage] = useState(0);
  const token = JSON.stringify([revision, current]);
  const refresh = (rank = candidate) => {
    setCandidate(rank);
    setSnapshot(current.map((n) => ({ ...n })));
    setSource(token);
    setChosen(new Set());
    setMeasure("all");
    setPage(0);
  };
  const before = new Map(snapshot.map((n) => [key(n), n]));
  const differences = (plans[candidate]?.proposals ?? []).filter(
    (n) => before.has(key(n)) && before.get(key(n))!.finger !== n.finger,
  );
  const filtered = differences.filter(
    (n) => measure === "all" || n.measure === Number(measure),
  );
  const measures = [...new Set(differences.map((n) => n.measure))].sort(
    (a, b) => a - b,
  );
  const stale = token !== source;
  const selected = differences.filter((n) => chosen.has(key(n)));
  return (
    <section className="finger-comparison" aria-label={label}>
      <button
        disabled={disabled || plans.length < 2}
        aria-expanded={open}
        onClick={() => {
          if (!open) refresh();
          setOpen(!open);
        }}
      >
        {open ? "收起逐音比较" : "逐音比较方案"}
      </button>
      {open && (
        <>
          <p>
            左列是开始比较时的当前编辑指法；查看另一方案不会替换它。只采用勾选差异，其他修改和未采用位置保留，随后核对实际保存结果。
          </p>
          <div className="finger-comparison-tools">
            <label>
              对照方案
              <select
                aria-label={`${label}对照方案`}
                value={candidate}
                disabled={disabled}
                onChange={(e) => refresh(Number(e.target.value))}
              >
                {plans.map((_, i) => (
                  <option key={i} value={i}>
                    方案 {i + 1}
                  </option>
                ))}
              </select>
            </label>
            <label>
              查看小节
              <select
                aria-label={`${label}小节`}
                value={measure}
                onChange={(e) => {
                  setMeasure(e.target.value);
                  setPage(0);
                }}
              >
                <option value="all">全部小节</option>
                {measures.map((m) => (
                  <option key={m} value={m}>
                    第 {m} 小节
                  </option>
                ))}
              </select>
            </label>
            <button disabled={disabled} onClick={() => refresh()}>
              重新比较当前编辑结果
            </button>
          </div>
          <p>
            {differences.length} 处差异 · 已选 {selected.length}{" "}
            处。对照方案：穿 / 跨指 {plans[candidate]?.crossings} 次、移手{" "}
            {plans[candidate]?.shifts} 次、宽手型 {plans[candidate]?.wide}{" "}
            处、改动已有 {plans[candidate]?.changed} 处。
          </p>
          {stale && (
            <p role="status">当前编辑或采用范围已变化，请重新比较后再采用。</p>
          )}
          <div className="finger-comparison-tools">
            <button
              disabled={disabled || stale || !filtered.length}
              onClick={() =>
                setChosen(new Set([...chosen, ...filtered.map(key)]))
              }
            >
              选择当前小节差异
            </button>
            <button
              disabled={disabled || !chosen.size}
              onClick={() => setChosen(new Set())}
            >
              清空比较选择
            </button>
            <button
              disabled={disabled || stale || !selected.length}
              onClick={() => apply(selected)}
            >
              采用选中差异（{selected.length}）
            </button>
          </div>
          <div className="finger-comparison-table">
            <table>
              <thead>
                <tr>
                  <th>采用</th>
                  <th>位置 / 音符</th>
                  <th>当前编辑</th>
                  <th>对照指法</th>
                  <th>推荐依据</th>
                </tr>
              </thead>
              <tbody>
                {filtered.slice(page * 12, (page + 1) * 12).map((n) => (
                  <tr key={key(n)}>
                    <td>
                      <input
                        type="checkbox"
                        aria-label={`${label}采用差异 ${key(n)}`}
                        disabled={disabled || stale}
                        checked={chosen.has(key(n))}
                        onChange={(e) =>
                          setChosen((s) => {
                            const v = new Set(s);
                            e.target.checked ? v.add(key(n)) : v.delete(key(n));
                            return v;
                          })
                        }
                      />
                    </td>
                    <td>
                      第 {n.measure} 小节 · {pitchName(n.pitch)}
                      <small>
                        {trackName(n.track)} · 音符 {n.index + 1}
                      </small>
                      <button disabled={disabled} onClick={() => locate(n)}>
                        定位当前指法
                      </button>
                    </td>
                    <td>{before.get(key(n))?.finger ?? "未标记"}</td>
                    <td>{n.finger}</td>
                    <td>{n.reason}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {!filtered.length && (
            <p>此范围与当前编辑指法一致，没有需要采用的差异。</p>
          )}
          {filtered.length > 12 && (
            <div className="finger-comparison-tools">
              <button disabled={!page} onClick={() => setPage(page - 1)}>
                上一页差异
              </button>
              <span>
                {page + 1} / {Math.ceil(filtered.length / 12)}
              </span>
              <button
                disabled={(page + 1) * 12 >= filtered.length}
                onClick={() => setPage(page + 1)}
              >
                下一页差异
              </button>
            </div>
          )}
        </>
      )}
    </section>
  );
}
