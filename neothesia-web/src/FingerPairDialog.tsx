import {
  FingerPlanComparison,
  type ComparedPlan,
} from "./FingerPlanComparison";
import {
  FingerPairInteractions,
  interactionInstruction,
  type PairInteractionAnalysis,
  type PairInteraction,
} from "./FingerPairInteractions";
import { FingerDemoControls } from "./FingerDemoControls";
import {
  FingerPracticeBuilder,
  type FingerDifficulty,
} from "./FingerPracticeBuilder";
import { useEffect, useRef, useState } from "react";
import { api, pitchName, type LoadedSong } from "./api";
type Proposal = {
  track: number;
  index: number;
  measure: number;
  pitch: number;
  finger: number;
  reason: string;
};
type Result = {
  request: Record<string, unknown>;
  fingerprint: string;
  plans: (ComparedPlan & { proposals: Proposal[]; cost: number })[];
};
type Review = {
  proof: string;
  valid: boolean;
  needsAck: boolean;
  selected: number;
  unassigned: number;
  interactions: PairInteractionAnalysis;
  shared: { measure: number; pitch: number }[];
  reviews: {
    selected: number;
    retained: number;
    unmarked: number;
    contextUnmarked: number;
    conflicts: { measure: number; reason: string }[];
    review: FingerDifficulty[];
  }[];
};
export function FingerPairDialog({
  song,
  start,
  end,
  close,
  changed,
}: {
  song: LoadedSong;
  start: number;
  end: number;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [first, setFirst] = useState(start),
    [last, setLast] = useState(end),
    [rate, setRate] = useState(1),
    [profiles, setProfiles] = useState(["Standard", "Standard"]),
    [results, setResults] = useState<Result[]>([]),
    [ranks, setRanks] = useState([0, 0]),
    [edits, setEdits] = useState<Record<string, number>[]>([{}, {}]),
    [selected, setSelected] = useState<Set<string>[]>([new Set(), new Set()]),
    [pages, setPages] = useState([0, 0]),
    [review, setReview] = useState<Review | null>(null),
    [ack, setAck] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const busyRef = useRef(false),
    root = useRef<HTMLElement>(null);
  const hands = ["left", "right"] as const;
  const key = (p: Proposal) => `${p.track}:${p.index}`;
  const profileTouched = useRef(false);
  useEffect(() => {
    let disposed = false;
    void api
      .command<{
        defaults: Record<string, string>;
        profilesByHand?: Record<string, string>;
      }>({ type: "fingerProfiles" })
      .then((data) => {
        if (!disposed && !profileTouched.current)
          setProfiles(
            hands.map((h) => {
              const note = song.notes.find((n) => n.part === h);
              return (
                data.profilesByHand?.[`${note?.track}:${h}`] ??
                data.defaults[h === "left" ? "LeftHand" : "RightHand"] ??
                "Standard"
              );
            }),
          );
      })
      .catch((e) => {
        if (!disposed) setError(String(e.message));
      });
    return () => {
      disposed = true;
    };
  }, [song.contentId]);
  useEffect(() => {
    const unload = (e: BeforeUnloadEvent) => {
      if (results.length) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", unload);
    return () => window.removeEventListener("beforeunload", unload);
  }, [results.length]);
  const dirty = results.length > 0;
  const exit = () => {
    if (
      !busyRef.current &&
      (!dirty || confirm("双手指法尚未保存，放弃当前方案？"))
    )
      close();
  };
  useEffect(() => {
    const prior = document.activeElement;
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        exit();
      }
      if (e.key === "Tab") {
        const els = [
          ...(root.current?.querySelectorAll<HTMLElement>(
            "button:not(:disabled),input:not(:disabled),select:not(:disabled)",
          ) ?? []),
        ].filter((n) => n.getClientRects().length);
        if (
          els.length &&
          ((e.shiftKey && document.activeElement === els[0]) ||
            (!e.shiftKey && document.activeElement === els.at(-1)))
        ) {
          e.preventDefault();
          (e.shiftKey ? els.at(-1) : els[0])?.focus();
        }
      }
    };
    document.addEventListener("keydown", escape, true);
    return () => {
      document.removeEventListener("keydown", escape, true);
      if (prior instanceof HTMLElement && prior.isConnected) prior.focus();
    };
  }, [dirty]);
  const invalidate = () => {
    setReview(null);
    setAck(false);
    setError("");
  };
  const requestFor = (hand: string, i: number) => {
    const notes = song.notes.filter((n) => n.part === hand);
    if (!notes.length)
      throw Error(`请先指定${hand === "left" ? "左" : "右"}手音符`);
    const note = notes[0];
    return {
      content_id: song.contentId,
      track: note.track,
      index: note.index,
      profile: profiles[i],
      hand: hand === "left" ? "LeftHand" : "RightHand",
      range: [first, last],
      keep_saved: true,
      practice_rate: rate,
      target_tracks: [...new Set(notes.map((n) => n.track))],
      pins: [],
    };
  };
  const parts = () =>
    results.map((r, i) => ({
      request: r.request,
      fingerprint: r.fingerprint,
      edits: r.plans[ranks[i]].proposals
        .filter((p) => selected[i].has(key(p)))
        .map((p) => ({
          track_id: p.track,
          note_index: p.index,
          finger: edits[i][key(p)] ?? p.finger,
        })),
    }));
  const work = async (job: () => Promise<void>) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError("");
    try {
      await job();
    } catch (e) {
      setReview(null);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };
  const generate = () =>
    void work(async () => {
      profileTouched.current = true;
      const requests = hands.map(requestFor);
      const rows = await Promise.all(
        requests.map((request) =>
          api.command<Result>({ type: "suggestFingerPlans", request }),
        ),
      );
      if (rows.some((r) => !r.plans.length))
        throw Error("有一只手未生成可用方案");
      setResults(rows);
      setRanks([0, 0]);
      setEdits([{}, {}]);
      setSelected(rows.map((r) => new Set(r.plans[0].proposals.map(key))));
      setPages([0, 0]);
      invalidate();
    });
  const focusInteraction = (item: PairInteraction) => {
    const ids = [
      `${item.leftTrack}:${item.leftIndex}`,
      `${item.rightTrack}:${item.rightIndex}`,
    ];
    const indices = results.map((r, i) =>
      r.plans[ranks[i]].proposals.findIndex((p) => key(p) === ids[i]),
    );
    setPages(
      pages.map((p, i) => (indices[i] < 0 ? p : Math.floor(indices[i] / 30))),
    );
    window.requestAnimationFrame(() => {
      const nodes = ids
        .map((id, i) =>
          root.current?.querySelector<HTMLInputElement>(
            `input[aria-label="${i === 0 ? "左" : "右"}手指法 ${id}"]`,
          ),
        )
        .filter((n): n is HTMLInputElement => !!n);
      nodes.forEach((n) => {
        n.closest("tr")?.animate(
          [
            { backgroundColor: "#717f4460" },
            { backgroundColor: "transparent" },
          ],
          { duration: 2000 },
        );
      });
      nodes.at(-1)?.focus();
      nodes.at(-1)?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  };
  const reset = () => {
    setResults([]);
    invalidate();
  };
  return (
    <div
      className="modal-backdrop"
      onClick={(e) => {
        e.stopPropagation();
        exit();
      }}
    >
      <section
        ref={root}
        className="settings-dialog finger-pair-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="finger-pair-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="finger-pair-title">双手指法审阅</h2>
            <p>{song.title}</p>
          </div>
          <button autoFocus disabled={busy} onClick={exit}>
            关闭
          </button>
        </div>
        <p className="parameter-help">
          各手联合自己的音轨推荐，再核对两手的保存结果。一笔保存和撤销；手交叉、身体动作与双手同音仍需人工审阅。
        </p>
        <div className="finger-pair-options">
          <label>
            起点小节
            <input
              aria-label="双手起点小节"
              type="number"
              min={1}
              value={first}
              disabled={busy}
              onChange={(e) => {
                setFirst(Number(e.target.value));
                reset();
              }}
            />
          </label>
          <label>
            终点小节
            <input
              aria-label="双手终点小节"
              type="number"
              min={1}
              value={last}
              disabled={busy}
              onChange={(e) => {
                setLast(Number(e.target.value));
                reset();
              }}
            />
          </label>
          <label>
            推荐速度（%）
            <input
              aria-label="双手推荐速度"
              type="number"
              min={25}
              max={200}
              value={Math.round(rate * 100)}
              disabled={busy}
              onChange={(e) => {
                setRate(Number(e.target.value) / 100);
                reset();
              }}
            />
          </label>
          {hands.map((h, i) => (
            <label key={h}>
              {i === 0 ? "左" : "右"}手跨度
              <select
                aria-label={`${i === 0 ? "左" : "右"}手推荐跨度`}
                value={profiles[i]}
                disabled={busy}
                onChange={(e) => {
                  profileTouched.current = true;
                  setProfiles(
                    profiles.map((p, j) => (i === j ? e.target.value : p)),
                  );
                  reset();
                }}
              >
                <option value="Compact">较小</option>
                <option value="Standard">标准</option>
                <option value="Large">较大</option>
              </select>
            </label>
          ))}
          <button disabled={busy} onClick={generate}>
            生成双手建议
          </button>
        </div>
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        <div className="finger-pair-hands">
          {results.map((r, i) => {
            const proposals = r.plans[ranks[i]].proposals;
            return (
              <section key={i}>
                <div className="finger-pair-hand-heading">
                  <h3>
                    {i === 0 ? "左" : "右"}手 · 已选 {selected[i].size}/
                    {proposals.length}
                  </h3>
                  <select
                    aria-label={`${i === 0 ? "左" : "右"}手方案`}
                    disabled={busy}
                    value={ranks[i]}
                    onChange={(e) => {
                      const rank = Number(e.target.value);
                      setRanks(ranks.map((v, j) => (i === j ? rank : v)));
                      setEdits(edits.map((v, j) => (i === j ? {} : v)));
                      setSelected(
                        selected.map((v, j) =>
                          i === j
                            ? new Set(r.plans[rank].proposals.map(key))
                            : v,
                        ),
                      );
                      invalidate();
                    }}
                  >
                    {r.plans.map((p, j) => (
                      <option key={j} value={j}>
                        方案 {j + 1} · 穿 / 跨指 {p.crossings} · 移手 {p.shifts}
                      </option>
                    ))}
                  </select>
                </div>
                <FingerPlanComparison
                  key={r.fingerprint}
                  label={`${i === 0 ? "左" : "右"}手方案比较`}
                  plans={r.plans}
                  current={proposals.map((p) => ({
                    ...p,
                    finger: edits[i][key(p)] ?? p.finger,
                  }))}
                  revision={JSON.stringify([
                    r.fingerprint,
                    ranks[i],
                    [...selected[i]].sort(),
                  ])}
                  disabled={busy}
                  trackName={(id) =>
                    song.tracks.find((t) => t.id === id)?.name ||
                    `音轨 ${id + 1}`
                  }
                  apply={(notes) => {
                    setEdits((v) =>
                      v.map((e, j) =>
                        j === i
                          ? {
                              ...e,
                              ...Object.fromEntries(
                                notes.map((n) => [key(n), n.finger]),
                              ),
                            }
                          : e,
                      ),
                    );
                    setSelected((v) =>
                      v.map((e, j) =>
                        j === i ? new Set([...e, ...notes.map(key)]) : e,
                      ),
                    );
                    invalidate();
                  }}
                  locate={(n) => {
                    const index = proposals.findIndex((p) => key(p) === key(n));
                    if (index < 0) return;
                    setPages((v) =>
                      v.map((p, j) => (j === i ? Math.floor(index / 30) : p)),
                    );
                    requestAnimationFrame(() => {
                      const input =
                        root.current?.querySelector<HTMLInputElement>(
                          `input[aria-label="${i === 0 ? "左" : "右"}手指法 ${key(n)}"]`,
                        );
                      input?.focus();
                      input?.scrollIntoView({ block: "center" });
                    });
                  }}
                />
                <div className="finger-pair-selection">
                  <button
                    disabled={busy}
                    onClick={() => {
                      setSelected(
                        selected.map((v, j) =>
                          i === j ? new Set(proposals.map(key)) : v,
                        ),
                      );
                      invalidate();
                    }}
                  >
                    全选
                  </button>
                  <button
                    disabled={busy}
                    onClick={() => {
                      setSelected(
                        selected.map((v, j) => (i === j ? new Set() : v)),
                      );
                      invalidate();
                    }}
                  >
                    清空选择
                  </button>
                </div>
                <table>
                  <thead>
                    <tr>
                      <th>采用</th>
                      <th>位置 / 音符</th>
                      <th>手指</th>
                    </tr>
                  </thead>
                  <tbody>
                    {proposals
                      .slice(pages[i] * 30, (pages[i] + 1) * 30)
                      .map((p) => (
                        <tr key={key(p)}>
                          <td>
                            <input
                              type="checkbox"
                              aria-label={`${i === 0 ? "左" : "右"}手采用 ${key(p)}`}
                              checked={selected[i].has(key(p))}
                              disabled={busy}
                              onChange={(e) => {
                                const next = new Set(selected[i]);
                                if (e.target.checked) next.add(key(p));
                                else next.delete(key(p));
                                setSelected(
                                  selected.map((v, j) => (i === j ? next : v)),
                                );
                                invalidate();
                              }}
                            />
                          </td>
                          <td>
                            第 {p.measure} 小节 · {pitchName(p.pitch)}
                            <small>
                              {song.tracks.find((t) => t.id === p.track)?.name}{" "}
                              · 音符 {p.index + 1}
                            </small>
                            <small>{p.reason}</small>
                          </td>
                          <td>
                            <input
                              type="number"
                              aria-label={`${i === 0 ? "左" : "右"}手指法 ${key(p)}`}
                              min={1}
                              max={5}
                              value={edits[i][key(p)] ?? p.finger}
                              disabled={busy}
                              onChange={(e) => {
                                setEdits(
                                  edits.map((v, j) =>
                                    i === j
                                      ? {
                                          ...v,
                                          [key(p)]: Number(e.target.value),
                                        }
                                      : v,
                                  ),
                                );
                                invalidate();
                              }}
                            />
                          </td>
                        </tr>
                      ))}
                  </tbody>
                </table>
                <div className="finger-pair-selection">
                  <button
                    disabled={busy || pages[i] === 0}
                    onClick={() =>
                      setPages(pages.map((v, j) => (j === i ? v - 1 : v)))
                    }
                  >
                    上一页
                  </button>
                  <span>
                    {pages[i] + 1}/
                    {Math.max(1, Math.ceil(proposals.length / 30))}
                  </span>
                  <button
                    disabled={busy || (pages[i] + 1) * 30 >= proposals.length}
                    onClick={() =>
                      setPages(pages.map((v, j) => (j === i ? v + 1 : v)))
                    }
                  >
                    下一页
                  </button>
                </div>
              </section>
            );
          })}
        </div>
        {review && (
          <section className="finger-pair-review" role="status">
            <strong>
              双手共保存 {review.selected} 个指法 ·{" "}
              {review.valid ? "手内冲突检查通过" : "存在冲突，双手均不可保存"}
            </strong>
            {review.reviews.map((r, i) => (
              <div key={i}>
                <p>
                  {i === 0 ? "左" : "右"}手：采用 {r.selected} · 保留{" "}
                  {r.retained} · 未标记 {r.unmarked} · 上下文未标记{" "}
                  {r.contextUnmarked} · 待复核 {r.review.length} 处
                </p>
                {r.conflicts.map((c, j) => (
                  <p key={j}>
                    第 {c.measure} 小节：{c.reason}
                  </p>
                ))}
                {r.review.length > 0 && (
                  <details>
                    <summary>查看 {r.review.length} 处跨度与换位复核</summary>
                    <div className="finger-pair-review-list">
                      {r.review.map((v, j) => (
                        <p key={j}>
                          第 {v.measure} 小节 ·{" "}
                          {{
                            wideReach: "跨度",
                            sameKeyHandoff: "同键交接",
                            crossedRange: "音域交叠",
                            rapidTurn: "快速换指",
                            rapidShift: "快速移手",
                            sameFingerMove: "同指移动",
                          }[v.kind] ?? "手位衔接"}
                        </p>
                      ))}
                    </div>
                  </details>
                )}
              </div>
            ))}
            <p>
              范围内未分手：{review.unassigned} 个音符；双手同时同音：
              {review.shared.length} 处（最多列计 100 处）。
            </p>
            {review.shared.slice(0, 8).map((v, i) => (
              <p key={i}>
                第 {v.measure} 小节 · {pitchName(v.pitch)}
                ：核对实际由哪只手演奏。
              </p>
            ))}
            <FingerPairInteractions
              key={review.proof}
              analysis={review.interactions}
              focus={focusInteraction}
              disabled={busy}
            />
            {review.needsAck && (
              <label>
                <input
                  type="checkbox"
                  aria-label="确认双手保留与待复核位置"
                  checked={ack}
                  disabled={busy}
                  onChange={(e) => setAck(e.target.checked)}
                />
                已审阅未选建议、未标记/未分手、同键持音交接和两手音域位置，按当前结果保存
              </label>
            )}
          </section>
        )}
        {review && (
          <FingerDemoControls
            result={{ fingerprint: review.proof, request: {}, context: [] }}
            measures={song.measures}
            pitches={song.notes
              .filter(
                (n) =>
                  (n.part === "left" || n.part === "right") &&
                  n.start < (song.measures[last - 1]?.end ?? song.duration) &&
                  n.start + n.duration > (song.measures[first - 1]?.start ?? 0),
              )
              .map((n) => n.pitch)}
            plan={review.proof}
            time={() => {}}
            active={() => {}}
            move={() => {}}
            disabled={busy || !review.valid || (review.needsAck && !ack)}
            pair={{ parts: parts(), proof: review.proof, acknowledged: ack }}
          />
        )}
        {review && (
          <FingerPracticeBuilder
            key={review.proof}
            review={[
              ...review.reviews.flatMap((r) => r.review),
              ...review.interactions.items.map((item) => ({
                measure: item.measure,
                fromMeasure: item.fromMeasure,
                pitch: item.rightPitch,
                fromPitch: item.leftPitch,
                kind: item.kind,
                intervalMs: null,
                semitones: Math.abs(item.leftPitch - item.rightPitch),
                comfortableSemitones: 0,
                instruction: interactionInstruction(item),
              })),
            ]}
            rate={rate}
            hand="both"
            request={{}}
            fingerprint=""
            measures={song.measures.length}
            disabled={busy || !review.valid || (review.needsAck && !ack)}
            pair={{ parts: parts(), proof: review.proof, acknowledged: ack }}
            fallback={[
              {
                selected: true,
                name: `双手衔接 · ${first}–${last} 小节`,
                start: first,
                end: last,
                speed: Math.min(rate, 0.75),
                rounds: 3,
                notes: "双手衔接慢练，分别保持手位与放松。使用当前已保存指法。",
              },
            ]}
            busyChanged={(v) => {
              busyRef.current = v;
              setBusy(v);
            }}
            changed={changed}
          />
        )}
        <div className="dialog-actions">
          <button
            disabled={busy || results.length !== 2}
            onClick={() =>
              void work(async () => {
                setAck(false);
                setReview(
                  await api.command<Review>({
                    type: "fingerPair",
                    parts: parts(),
                    proof: null,
                    acknowledged: false,
                  }),
                );
              })
            }
          >
            核对双手保存结果
          </button>
          <button
            className="primary"
            disabled={busy || !review?.valid || (review.needsAck && !ack)}
            onClick={() =>
              void work(async () => {
                await api.command({
                  type: "fingerPair",
                  parts: parts(),
                  proof: review?.proof,
                  acknowledged: ack,
                });
                await changed();
                close();
              })
            }
          >
            保存双手指法
          </button>
          <button disabled={busy} onClick={exit}>
            取消
          </button>
        </div>
      </section>
    </div>
  );
}
