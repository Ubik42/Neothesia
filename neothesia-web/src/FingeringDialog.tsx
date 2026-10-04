import { FingerPlanComparison } from "./FingerPlanComparison";
import { FingerPairDialog } from "./FingerPairDialog";
import { useEffect, useState, useRef, useMemo } from "react";
import { X, ChevronLeft, ChevronRight } from "lucide-react";
import { FingerDemoControls } from "./FingerDemoControls";
import { FingerPracticeBuilder } from "./FingerPracticeBuilder";
import { HeldFingerPlans } from "./HeldFingerPlans";
import { api, pitchName, type LoadedSong, type Note } from "./api";
type Proposal = {
  track: number;
  index: number;
  pitch: number;
  measure: number;
  finger: number;
  saved: number | null;
  confidence: number;
  reason: string;
};
type Plan = {
  cost: number;
  proposals: Proposal[];
  contextFingers: number[];
  crossings: number;
  shifts: number;
  wide: number;
  changed: number;
  practiceRate: number;
  review: {
    track: number;
    fromTrack: number;
    contextIndex: number;
    index: number;
    measure: number;
    fromMeasure: number;
    pitch: number;
    fromPitch: number;
    kind: "wideReach" | "rapidTurn" | "rapidShift" | "sameFingerMove";
    semitones: number;
    comfortableSemitones: number;
    intervalMs: number | null;
  }[];
};
type Result = {
  fingerprint: string;
  request: Record<string, unknown>;
  plans: Plan[];
  context: {
    track: number;
    pitch: number;
    index: number;
    onset: number;
    end: number;
    measure: number;
    target: boolean;
  }[];
  contentId: string;
  proposals: Proposal[];
  undoAvailable: boolean;
  contextTracks: number[];
  targetTracks: number[];
};
type AcceptanceReview = {
  key: string;
  valid: boolean;
  selected: number;
  retained: number;
  unmarked: number;
  contextUnmarked: number;
  total: number;
  truncated: boolean;
  reviewTruncated: boolean;
  conflicts: {
    contextIndex: number;
    fromContextIndex: number;
    measure: number;
    fromMeasure: number;
    track: number;
    fromTrack: number;
    pitch: number;
    fromPitch: number;
    finger: number;
    fromFinger: number;
    reason: string;
  }[];
  review: Plan["review"];
  contextFingers: (number | null)[];
  tracks: {
    track: number;
    total: number;
    selected: number;
    retained: number;
    unmarked: number;
    changed: number;
    new: number;
  }[];
};
export function FingeringDialog({
  song,
  note,
  start,
  end,
  close,
  changed,
}: {
  song: LoadedSong;
  note: Note | null;
  start: number;
  end: number;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const playable = song.tracks.filter((t) => t.notes > 0);
  const trackName = (id: number) =>
    song.tracks.find((t) => t.id === id)?.name || `音轨 ${id + 1}`;
  const [track, setTrack] = useState(
    note?.track ??
      playable.find((t) => t.part !== "other")?.id ??
      playable[0]?.id ??
      0,
  );
  const [hand, setHand] = useState(
    note?.part ??
      song.notes.find((n) => n.track === track && n.part !== "other")?.part ??
      "right",
  );
  const [pair, setPair] = useState(false);
  const [profile, setProfile] = useState("Standard"),
    [scope, setScope] = useState(note ? "note" : "range"),
    [first, setFirst] = useState(start),
    [last, setLast] = useState(end);
  const [result, setResult] = useState<Result | null>(null),
    [accepted, setAccepted] = useState<Set<number>>(new Set()),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [page, setPage] = useState(0);
  const [values, setValues] = useState<Record<string, number>>({});
  const [additionalTracks, setAdditionalTracks] = useState<number[]>([]);
  const targetSignature = additionalTracks.join(",");
  const candidates = playable.filter(
    (t) =>
      t.id !== track &&
      song.notes.some((n) => n.track === t.id && n.part === hand),
  );
  const positionKey = (n: { track: number; index: number }) =>
    `${n.track}:${n.index}`;
  const isBatch =
    (result?.targetTracks.length ?? additionalTracks.length + 1) > 1;
  const noteLabel = (n: { track: number; index: number }) =>
    `${isBatch ? `${trackName(n.track)} · ` : ""}音符 ${n.index + 1}`;
  useEffect(() => {
    setAdditionalTracks([]);
  }, [track, hand, song.contentId]);
  const [planIndex, setPlanIndex] = useState(0);
  const [keepSaved, setKeepSaved] = useState(!note);
  const [preview, setPreview] = useState(0);
  const [practiceRate, setPracticeRate] = useState(1);
  const [reviewPage, setReviewPage] = useState(0);
  const [acceptance, setAcceptance] = useState<AcceptanceReview | null>(null);
  const [acceptanceError, setAcceptanceError] = useState("");
  const [acknowledged, setAcknowledged] = useState(false);
  const [acceptancePage, setAcceptancePage] = useState(0);
  const [previewSaved, setPreviewSaved] = useState(false);
  const acceptanceEdits = useMemo(
    () =>
      result?.proposals
        .filter((_, i) => accepted.has(i))
        .map((p) => ({
          track_id: p.track,
          note_index: p.index,
          finger: p.finger,
        })) ?? [],
    [result, accepted],
  );
  const acceptanceKey = JSON.stringify([result?.fingerprint, acceptanceEdits]);
  const hasChanges = Object.keys(values).length > 0;
  const currentAcceptance =
    acceptance?.key === acceptanceKey && !hasChanges ? acceptance : null;
  useEffect(() => {
    let disposed = false;
    setAcceptance(null);
    setAcknowledged(false);
    setAcceptancePage(0);
    setAcceptanceError("");
    setPreviewSaved(false);
    if (!result || !acceptanceEdits.length || hasChanges) return;
    void api
      .command<Omit<AcceptanceReview, "key">>({
        type: "reviewFingerAcceptance",
        request: result.request,
        fingerprint: result.fingerprint,
        edits: acceptanceEdits,
      })
      .then((data) => {
        if (!disposed) setAcceptance({ ...data, key: acceptanceKey });
      })
      .catch((e) => {
        if (!disposed)
          setAcceptanceError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      disposed = true;
    };
  }, [acceptanceKey, hasChanges]);
  const profileChanged = useRef(false);
  useEffect(() => {
    const handle = (e: KeyboardEvent) => {
      if (e.key === "Escape" && busy) {
        e.stopImmediatePropagation();
        e.preventDefault();
      }
    };
    window.addEventListener("keydown", handle, true);
    return () => window.removeEventListener("keydown", handle, true);
  }, [busy]);
  useEffect(() => {
    let cancelled = false;
    profileChanged.current = false;
    void api
      .command<{
        profiles: Record<string, string> | null;
        profilesByHand: Record<string, string> | null;
        defaults: Record<string, string>;
      }>({
        type: "fingerProfiles",
      })
      .then((data) => {
        if (!cancelled && !profileChanged.current)
          setProfile(
            data.profilesByHand?.[`${track}:${hand}`] ??
              data.defaults[hand === "left" ? "LeftHand" : "RightHand"] ??
              data.profiles?.[String(track)] ??
              "Standard",
          );
      })
      .catch((e) => {
        if (!cancelled) setError(String(e.message));
      });
    return () => {
      cancelled = true;
    };
  }, [track, hand, song.contentId]);
  useEffect(() => {
    setResult(null);
    setValues({});
    setAccepted(new Set());
    setPage(0);
  }, [
    track,
    hand,
    profile,
    scope,
    first,
    last,
    keepSaved,
    practiceRate,
    targetSignature,
  ]);
  const perform = async (work: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await work();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const generate = () =>
    perform(async () => {
      const index =
        note?.track === track
          ? note.index
          : (song.notes.find((n) => n.track === track)?.index ?? 0);
      const data = await api.command<Result>({
        type: "suggestFingerPlans",
        request: {
          content_id: song.contentId,
          track,
          index,
          profile,
          hand: hand === "left" ? "LeftHand" : "RightHand",
          range: scope === "range" ? [first, last] : null,
          keep_saved: keepSaved,
          practice_rate: practiceRate,
          target_tracks: additionalTracks,
          pins: Object.entries({
            ...Object.fromEntries(
              (
                (result?.request.pins ?? []) as {
                  track?: number | null;
                  index: number;
                  finger: number;
                }[]
              ).map((p) => [`${p.track ?? track}:${p.index}`, p.finger]),
            ),
            ...values,
          }).map(([key, finger]) => ({
            track: Number(key.split(":")[0]),
            index: Number(key.split(":")[1]),
            finger,
          })),
        },
      });
      data.proposals = data.plans[0]?.proposals ?? [];
      setResult(data);
      setPlanIndex(0);
      setPreview(0);
      setReviewPage(0);
      setValues({});
      setPage(0);
      const previouslyAccepted =
        result && Object.keys(values).length > 0
          ? new Set(
              result.proposals
                .filter((_, i) => accepted.has(i))
                .map(positionKey),
            )
          : null;
      setAccepted(
        new Set(
          data.proposals.flatMap((p, i) =>
            !previouslyAccepted || previouslyAccepted.has(positionKey(p))
              ? [i]
              : [],
          ),
        ),
      );
    });
  const apply = () =>
    perform(async () => {
      if (
        !result ||
        !currentAcceptance?.valid ||
        ((currentAcceptance.selected < currentAcceptance.total ||
          currentAcceptance.contextUnmarked > 0) &&
          !acknowledged)
      )
        return;
      await api.command({
        type: "acceptFingerPlan",
        request: result.request,
        fingerprint: result.fingerprint,
        edits: acceptanceEdits,
      });
      await changed();
      close();
    });
  const visible = result?.proposals.slice(page * 40, (page + 1) * 40) ?? [];
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog fingering-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="fingering-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="fingering-title">指法建议</h2>
            <p>{song.title}</p>
          </div>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭指法建议"
            onClick={() => {
              if (!busy) close();
            }}
          >
            <X size={18} />
          </button>
        </div>
        {pair && (
          <FingerPairDialog
            song={song}
            start={first}
            end={last}
            close={() => setPair(false)}
            changed={async () => {
              setResult(null);
              await changed();
            }}
          />
        )}
        <button disabled={busy} onClick={() => setPair(true)}>
          双手一起审阅
        </button>
        <div className="fingering-options">
          <label>
            音轨
            <select
              aria-label="建议指法音轨"
              value={track}
              disabled={busy}
              onChange={(e) => {
                const id = Number(e.target.value);
                setTrack(id);
                const part = song.notes.find(
                  (n) => n.track === id && n.part !== "other",
                )?.part;
                if (part) setHand(part);
              }}
            >
              {playable.map((t) => (
                <option value={t.id} key={t.id}>
                  {t.name} ·{" "}
                  {t.part === "left"
                    ? "左手"
                    : t.part === "right"
                      ? "右手"
                      : "未分手"}
                </option>
              ))}
            </select>
          </label>
          <label>
            演奏手
            <select
              aria-label="指法演奏手"
              value={hand}
              disabled={busy}
              onChange={(e) => setHand(e.target.value as "left" | "right")}
            >
              <option value="left">左手</option>
              <option value="right">右手</option>
            </select>
          </label>
          <label>
            手的跨度
            <select
              aria-label="指法手跨度"
              value={profile}
              disabled={busy}
              onChange={(e) => {
                profileChanged.current = true;
                setProfile(e.target.value);
              }}
            >
              <option value="Compact">较小 · 七度内</option>
              <option value="Standard">一般 · 八度内</option>
              <option value="Large">较大 · 九度内</option>
            </select>
          </label>
          <button
            disabled={busy}
            onClick={() =>
              void perform(async () => {
                await api.command({
                  type: "handSpanDefault",
                  hand: hand === "left" ? "LeftHand" : "RightHand",
                  profile,
                });
              })
            }
          >
            设为{hand === "left" ? "左手" : "右手"}默认跨度
          </button>
          <label>
            建议范围
            <select
              aria-label="指法建议范围"
              value={scope}
              disabled={busy}
              onChange={(e) => setScope(e.target.value)}
            >
              {note && <option value="note">所选音符 / 整个和弦</option>}
              <option value="range">小节选段</option>
            </select>
          </label>
          <label>
            推荐使用速度
            <select
              aria-label="指法推荐速度"
              value={practiceRate}
              disabled={busy}
              onChange={(e) => setPracticeRate(Number(e.target.value))}
            >
              {[0.25, 0.5, 0.75, 1, 1.25, 1.5, 2].map((rate) => (
                <option key={rate} value={rate}>
                  {Math.round(rate * 100)}% 原速
                </option>
              ))}
            </select>
          </label>
          {scope === "range" && (
            <>
              <label>
                起始小节
                <input
                  disabled={busy}
                  aria-label="指法起始小节"
                  type="number"
                  min={1}
                  max={song.measures.length}
                  value={first}
                  onChange={(e) => setFirst(Number(e.target.value))}
                />
              </label>
              <label>
                结束小节
                <input
                  disabled={busy}
                  aria-label="指法结束小节"
                  type="number"
                  min={first}
                  max={song.measures.length}
                  value={last}
                  onChange={(e) => setLast(Number(e.target.value))}
                />
              </label>
            </>
          )}
          <button
            className="primary-button"
            disabled={busy}
            onClick={() => void generate()}
          >
            {busy ? "处理中…" : "生成建议"}
          </button>
        </div>
        <HeldFingerPlans
          onBusy={setBusy}
          song={song}
          disabled={busy}
          changed={changed}
          request={{
            content_id: song.contentId,
            track,
            index:
              note?.track === track
                ? note.index
                : (song.notes.find((n) => n.track === track)?.index ?? 0),
            hand: hand === "left" ? "LeftHand" : "RightHand",
            profile,
            range: scope === "range" ? [first, last] : null,
            keep_saved: keepSaved,
            practice_rate: practiceRate,
            target_tracks: additionalTracks,
            pins: Object.entries(values).map(([key, finger]) => ({
              track: Number(key.split(":")[0]),
              index: Number(key.split(":")[1]),
              finger,
            })),
          }}
        />
        {candidates.length > 0 && (
          <fieldset className="finger-batch-targets" disabled={busy}>
            <legend>共同编辑的同手音轨</legend>
            <span>{trackName(track)} 已选</span>
            {candidates.map((t) => (
              <label key={t.id}>
                <input
                  type="checkbox"
                  aria-label={`共同编辑 ${trackName(t.id)}`}
                  checked={additionalTracks.includes(t.id)}
                  onChange={(e) =>
                    setAdditionalTracks((current) =>
                      (e.target.checked
                        ? [...current, t.id]
                        : current.filter((id) => id !== t.id)
                      ).sort((a, b) => a - b),
                    )
                  }
                />
                {trackName(t.id)}
              </label>
            ))}
            <small>
              勾选后一起推荐和保存；未勾选声部只作联动参考，其已有指法保持固定。
            </small>
          </fieldset>
        )}
        <label className="finger-anchor-toggle">
          <input
            type="checkbox"
            disabled={busy}
            checked={keepSaved}
            onChange={(e) => {
              setKeepSaved(e.target.checked);
              setValues({});
            }}
          />
          固定选段内已有指法
        </label>
        {!song.notes.some((n) => n.track === track && n.part !== "other") && (
          <div className="fingering-hand-setup">
            <span>这条音轨还未指定手。先确认其演奏手，再生成建议。</span>
            {[
              ["left", "指定左手"],
              ["right", "指定右手"],
            ].map(([part, label]) => (
              <button
                key={part}
                disabled={busy}
                onClick={() =>
                  void perform(async () => {
                    const t = playable.find((t) => t.id === track)!;
                    await api.command({
                      type: "track",
                      id: track,
                      mode: t.mode,
                      part,
                      visible: t.visible,
                    });
                    await changed();
                    setResult(null);
                  })
                }
              >
                {label}
              </button>
            ))}
          </div>
        )}
        <p className="fingering-help">
          综合旋律走向、黑白键、手型、穿指、和弦连接、持音与演奏速度。推荐使用速度影响换指代价，不改变当前练习速度。选段外已有指法始终保留。规则评分用于比较建议，不代表正确率；接受后才保存。
        </p>
        {error && (
          <p role="alert" className="dialog-error">
            {error}
          </p>
        )}
        {result ? (
          <>
            <div
              className="finger-plan-options"
              role="group"
              aria-label="备选指法方案"
            >
              {result.plans.map((plan, i) => (
                <button
                  key={i}
                  disabled={busy || Object.keys(values).length > 0}
                  aria-pressed={i === planIndex}
                  onClick={() => {
                    setPlanIndex(i);
                    setResult({ ...result, proposals: plan.proposals });
                    setAccepted(new Set(plan.proposals.map((_, j) => j)));
                    setPage(0);
                    setReviewPage(0);
                  }}
                >
                  <strong>方案 {i + 1}</strong>
                  <span>需复核 {plan.review.length} 处</span>
                  <small>
                    穿 / 跨指 {plan.crossings} · 移手 {plan.shifts} · 宽手型{" "}
                    {plan.wide} · 改动已有 {plan.changed}
                  </small>
                </button>
              ))}
            </div>
            <FingerPlanComparison
              key={result.fingerprint}
              label="指法方案比较"
              plans={result.plans}
              current={result.proposals.map((p) => ({
                ...p,
                finger: values[positionKey(p)] ?? p.finger,
              }))}
              revision={JSON.stringify([
                result.fingerprint,
                planIndex,
                [...accepted].sort((a, b) => a - b),
              ])}
              disabled={busy}
              trackName={trackName}
              apply={(notes) => {
                setValues((v) => ({
                  ...v,
                  ...Object.fromEntries(
                    notes.map((n) => [positionKey(n), n.finger]),
                  ),
                }));
                const keys = new Set(notes.map(positionKey));
                setAccepted(
                  (a) =>
                    new Set([
                      ...a,
                      ...result.proposals.flatMap((p, i) =>
                        keys.has(positionKey(p)) ? [i] : [],
                      ),
                    ]),
                );
              }}
              locate={(n) => {
                const i = result.proposals.findIndex(
                  (p) => positionKey(p) === positionKey(n),
                );
                if (i < 0) return;
                setPage(Math.floor(i / 40));
                requestAnimationFrame(() => {
                  const input = document.querySelector<HTMLSelectElement>(
                    `select[data-finger-position="${positionKey(n)}"]`,
                  );
                  input?.focus();
                  input?.scrollIntoView({ block: "center" });
                });
              }}
            />
            {Object.keys(values).length > 0 && (
              <p className="fingering-help">
                已保留逐音修改。点击“按修改重新推荐”将这些手指固定后重新求解，再核对实际保存结果；未勾选位置继续保持未采用。
              </p>
            )}
            {result.plans.length === 1 && (
              <p className="fingering-help">
                当前锚点和持音条件下只找到一个不同方案。可取消固定已有指法，重新比较。
              </p>
            )}
            {result.contextTracks.length > 0 && (
              <p className="fingering-help">
                联合参考同一只手的音轨：
                {result.contextTracks.map(trackName).join("、")}
                ，包含同时发音与持音；本次只保存{" "}
                {result.targetTracks.map(trackName).join("、")}{" "}
                的勾选建议。未选择音轨的已有指法保持固定，未标记指法仅参与方案预览。
              </p>
            )}
            {currentAcceptance && (
              <div
                className="fingering-selection"
                role="group"
                aria-label="手位预览内容"
              >
                <span>手位查看</span>
                {[false, true].map((saved) => (
                  <button
                    key={String(saved)}
                    disabled={busy}
                    aria-pressed={previewSaved === saved}
                    onClick={() => {
                      void api
                        .command({ type: "stopFingerDemo" })
                        .catch((e) => setError(String(e)));
                      setPreviewSaved(saved);
                    }}
                  >
                    {saved ? "保存后的实际指法" : "完整推荐方案"}
                  </button>
                ))}
              </div>
            )}
            <FingerPosition
              result={result}
              plan={result.plans[planIndex]}
              preview={preview}
              setPreview={setPreview}
              hand={hand}
              disabled={busy || Object.keys(values).length > 0}
              previewFingers={
                previewSaved ? currentAcceptance?.contextFingers : undefined
              }
            />
            <section className="finger-review" aria-label="指法难点复核">
              <div className="finger-review-heading">
                <strong>
                  需要复核的位置 · {result.plans[planIndex].review.length} 处
                </strong>
                <span>
                  {Math.round(result.plans[planIndex].practiceRate * 100)}% 原速
                </span>
              </div>
              <p>
                跨度按所选手型估算；快速穿 / 跨指以 180 毫秒、移手以 250
                毫秒为复核阈值。请结合手位示范判断，固定指法也会检查。
              </p>
              <FingerPracticeBuilder
                key={`${result.fingerprint}:${planIndex}`}
                review={result.plans[planIndex].review}
                rate={result.plans[planIndex].practiceRate}
                hand={hand}
                request={result.request}
                fingerprint={result.fingerprint}
                measures={song.measures.length}
                disabled={busy || Object.keys(values).length > 0}
                changed={changed}
              />
              {result.plans[planIndex].review
                .slice(reviewPage * 8, (reviewPage + 1) * 8)
                .map((r, i) => (
                  <button
                    key={`${r.index}:${r.kind}:${i}`}
                    disabled={busy || Object.keys(values).length > 0}
                    onClick={() => {
                      void api
                        .command({ type: "stopFingerDemo" })
                        .catch((e) => setError(String(e)));
                      setPreview(r.contextIndex);
                      const row = result.proposals.findIndex(
                        (p) => p.track === r.track && p.index === r.index,
                      );
                      if (row >= 0) setPage(Math.floor(row / 40));
                    }}
                  >
                    <span>
                      {trackName(r.fromTrack)} → {trackName(r.track)} · 第{" "}
                      {r.measure} 小节 · {pitchName(r.fromPitch)} →{" "}
                      {pitchName(r.pitch)}
                    </span>
                    <strong>
                      {r.kind === "wideReach"
                        ? `跨度 ${r.semitones} 半音，当前手型 ${r.comfortableSemitones} 半音`
                        : `${r.kind === "rapidTurn" ? "快速穿 / 跨指" : r.kind === "rapidShift" ? "快速移手" : "同指换键"} · ${r.intervalMs} 毫秒`}
                    </strong>
                    <small>
                      {r.kind === "wideReach"
                        ? "查看持音与手位；可调整分手、指法或改用分解演奏"
                        : "查看前后手位；可降低速度，或固定手指后重新推荐"}
                    </small>
                  </button>
                ))}
              {!result.plans[planIndex].review.length && (
                <span>当前规则没有发现上述难点；仍可逐位置审阅。</span>
              )}
              {result.plans[planIndex].review.length > 8 && (
                <div className="fingering-pages">
                  <button
                    aria-label="上一页指法难点"
                    disabled={!reviewPage}
                    onClick={() => setReviewPage(reviewPage - 1)}
                  >
                    <ChevronLeft size={16} />
                  </button>
                  <span>
                    {reviewPage + 1} /{" "}
                    {Math.ceil(result.plans[planIndex].review.length / 8)}
                  </span>
                  <button
                    aria-label="下一页指法难点"
                    disabled={
                      (reviewPage + 1) * 8 >=
                      result.plans[planIndex].review.length
                    }
                    onClick={() => setReviewPage(reviewPage + 1)}
                  >
                    <ChevronRight size={16} />
                  </button>
                </div>
              )}
            </section>
            {Object.keys(values).length > 0 && (
              <div className="fingering-selection">
                <span>
                  已固定 {Object.keys(values).length}{" "}
                  个手指；重新计算后才能保存。
                </span>
                <button disabled={busy} onClick={() => void generate()}>
                  按修改重新推荐
                </button>
                <button disabled={busy} onClick={() => setValues({})}>
                  放弃未计算的修改
                </button>
              </div>
            )}
            <div className="fingering-selection">
              <label>
                <input
                  type="checkbox"
                  aria-label="选择全部指法建议"
                  disabled={busy}
                  checked={
                    accepted.size === result.proposals.length &&
                    accepted.size > 0
                  }
                  onChange={(e) =>
                    setAccepted(
                      new Set(
                        e.target.checked
                          ? result.proposals.map((_, i) => i)
                          : [],
                      ),
                    )
                  }
                />
                全选 {result.proposals.length} 个音符
              </label>
              <span>
                已选择 {accepted.size} 个 ·
                只保存勾选音符，未勾选的位置保留已有标记
              </span>
            </div>
            <section
              className="finger-review finger-acceptance"
              aria-label="指法保存结果审阅"
            >
              <strong>保存后的实际指法</strong>
              {!currentAcceptance && !acceptanceError && (
                <p>
                  {hasChanges
                    ? "修改手指后，请先重新推荐。"
                    : accepted.size
                      ? "正在核对勾选建议与保留指法…"
                      : "勾选要接受的音符后审阅保存结果。"}
                </p>
              )}
              {acceptanceError && <p role="alert">{acceptanceError}</p>}
              {currentAcceptance && (
                <>
                  <p>
                    接受 {currentAcceptance.selected} 个 · 保留已有{" "}
                    {currentAcceptance.retained} 个 · 未标记{" "}
                    {currentAcceptance.unmarked} 个
                  </p>
                  {currentAcceptance.tracks.length > 1 && (
                    <div className="finger-batch-summary">
                      {currentAcceptance.tracks.map((summary) => (
                        <div key={summary.track}>
                          <strong>{trackName(summary.track)}</strong>
                          <span>
                            范围内 {summary.total} 个 · 接受 {summary.selected}{" "}
                            个 · 保留 {summary.retained} 个 · 未标记{" "}
                            {summary.unmarked} 个 · 新标记 {summary.new} 个 ·
                            改已有 {summary.changed} 个
                          </span>
                          <button
                            disabled={busy || !summary.total}
                            onClick={() =>
                              setAccepted((current) => {
                                const next = new Set(current);
                                result.proposals.forEach((p, i) => {
                                  if (p.track === summary.track)
                                    summary.selected === summary.total
                                      ? next.delete(i)
                                      : next.add(i);
                                });
                                return next;
                              })
                            }
                          >
                            {summary.selected === summary.total
                              ? "取消该轨选择"
                              : "接受该轨全部"}
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                  <p>
                    检查所选手在各音轨的同时按键及已标记位置。未标记位置仍无指法，不会填入未勾选的建议。
                  </p>
                  {currentAcceptance.contextUnmarked > 0 && (
                    <p>
                      同手其他音轨在本选段还有{" "}
                      {currentAcceptance.contextUnmarked}{" "}
                      个未标记位置；未保存的参考指法不能保证实际衔接。
                    </p>
                  )}
                  {!currentAcceptance.valid && (
                    <p role="alert">
                      勾选建议与保留指法发生冲突，这笔推荐不能保存。请调整选择，或固定已有指法重新推荐。
                    </p>
                  )}
                  {currentAcceptance.conflicts
                    .slice(acceptancePage * 8, (acceptancePage + 1) * 8)
                    .map((r, i) => (
                      <button
                        key={i}
                        disabled={busy}
                        onClick={() => {
                          void api
                            .command({ type: "stopFingerDemo" })
                            .catch((e) => setError(String(e)));
                          setPreview(r.contextIndex);
                          setPreviewSaved(true);
                          const position = result.context[r.contextIndex];
                          const row = result.proposals.findIndex(
                            (p) =>
                              p.track === position?.track &&
                              p.index === position?.index,
                          );
                          if (row >= 0) setPage(Math.floor(row / 40));
                        }}
                      >
                        <span>
                          {trackName(r.fromTrack)} 第 {r.fromMeasure} 小节{" "}
                          {pitchName(r.fromPitch)}（{r.fromFinger} 指）与{" "}
                          {trackName(r.track)} 第 {r.measure} 小节{" "}
                          {pitchName(r.pitch)}（{r.finger} 指）
                        </span>
                        <strong>{r.reason}</strong>
                      </button>
                    ))}
                  {currentAcceptance.conflicts.length > 8 && (
                    <div className="fingering-pages">
                      <button
                        aria-label="上一页保存冲突"
                        disabled={!acceptancePage}
                        onClick={() => setAcceptancePage(acceptancePage - 1)}
                      >
                        <ChevronLeft size={16} />
                      </button>
                      <span>
                        {acceptancePage + 1} /{" "}
                        {Math.ceil(currentAcceptance.conflicts.length / 8)}
                      </span>
                      <button
                        aria-label="下一页保存冲突"
                        disabled={
                          (acceptancePage + 1) * 8 >=
                          currentAcceptance.conflicts.length
                        }
                        onClick={() => setAcceptancePage(acceptancePage + 1)}
                      >
                        <ChevronRight size={16} />
                      </button>
                    </div>
                  )}
                  {currentAcceptance.truncated && (
                    <p>冲突较多，目前显示前 100 处；调整后重新检查。</p>
                  )}
                  {currentAcceptance.valid && (
                    <p>
                      勾选指法与已标记位置没有发现同时按键冲突。实际保存结果还有{" "}
                      {currentAcceptance.review.length}{" "}
                      处跨度或快速换指需要复核；原方案的难点不代替局部保存结果。
                    </p>
                  )}
                  {currentAcceptance.valid &&
                    currentAcceptance.review.length > 0 && (
                      <FingerPracticeBuilder
                        key={currentAcceptance.key}
                        review={currentAcceptance.review}
                        rate={result.plans[planIndex].practiceRate}
                        hand={hand}
                        request={result.request}
                        fingerprint={result.fingerprint}
                        measures={song.measures.length}
                        disabled={busy || Object.keys(values).length > 0}
                        changed={changed}
                      />
                    )}
                  {currentAcceptance.valid &&
                    currentAcceptance.review
                      .slice(acceptancePage * 8, (acceptancePage + 1) * 8)
                      .map((r, i) => (
                        <button
                          key={i}
                          disabled={busy}
                          onClick={() => {
                            void api
                              .command({ type: "stopFingerDemo" })
                              .catch((e) => setError(String(e)));
                            setPreview(r.contextIndex);
                            setPreviewSaved(true);
                          }}
                        >
                          <span>
                            第 {r.measure} 小节 · {pitchName(r.fromPitch)} →{" "}
                            {pitchName(r.pitch)}
                          </span>
                          <strong>
                            {r.kind === "wideReach"
                              ? `跨度 ${r.semitones} 半音，当前手型 ${r.comfortableSemitones} 半音`
                              : `换指需复核 · ${r.intervalMs} 毫秒`}
                          </strong>
                        </button>
                      ))}
                  {currentAcceptance.valid &&
                    currentAcceptance.review.length > 8 && (
                      <div className="fingering-pages">
                        <button
                          aria-label="上一页保存难点"
                          disabled={!acceptancePage}
                          onClick={() => setAcceptancePage(acceptancePage - 1)}
                        >
                          <ChevronLeft size={16} />
                        </button>
                        <span>
                          {acceptancePage + 1} /{" "}
                          {Math.ceil(currentAcceptance.review.length / 8)}
                        </span>
                        <button
                          aria-label="下一页保存难点"
                          disabled={
                            (acceptancePage + 1) * 8 >=
                            currentAcceptance.review.length
                          }
                          onClick={() => setAcceptancePage(acceptancePage + 1)}
                        >
                          <ChevronRight size={16} />
                        </button>
                      </div>
                    )}
                  {currentAcceptance.reviewTruncated && (
                    <p>保存难点较多，目前列出前 100 处；可缩小选段继续审阅。</p>
                  )}
                  {!currentAcceptance.valid && (
                    <button
                      disabled={busy}
                      onClick={() => {
                        setKeepSaved(true);
                        setValues({});
                        setResult(null);
                      }}
                    >
                      固定已有指法后重新选择建议
                    </button>
                  )}
                  {(currentAcceptance.selected < currentAcceptance.total ||
                    currentAcceptance.contextUnmarked > 0) && (
                    <label className="finger-anchor-toggle">
                      <input
                        type="checkbox"
                        checked={acknowledged}
                        disabled={busy || !currentAcceptance.valid}
                        onChange={(e) => setAcknowledged(e.target.checked)}
                      />
                      已审阅保留指法和未标记位置，只保存勾选音符
                    </label>
                  )}
                </>
              )}
            </section>
            <div className="fingering-table-wrap">
              <table className="fingering-table">
                <thead>
                  <tr>
                    <th>接受</th>
                    {isBatch && <th>音轨</th>}
                    <th>小节</th>
                    <th>音符</th>
                    <th>已有</th>
                    <th>建议</th>
                    <th>规则评分</th>
                    <th>原因</th>
                  </tr>
                </thead>
                <tbody>
                  {visible.map((p, j) => {
                    const i = page * 40 + j;
                    return (
                      <tr
                        key={positionKey(p)}
                        className={
                          result.context.find(
                            (n) => n.track === p.track && n.index === p.index,
                          )?.onset === result.context[preview]?.onset
                            ? "finger-preview-row"
                            : ""
                        }
                      >
                        <td>
                          <input
                            type="checkbox"
                            aria-label={`接受${noteLabel(p)} 指法`}
                            disabled={busy}
                            checked={accepted.has(i)}
                            onChange={(e) =>
                              setAccepted((s) => {
                                const next = new Set(s);
                                e.target.checked ? next.add(i) : next.delete(i);
                                return next;
                              })
                            }
                          />
                        </td>
                        {isBatch && <td>{trackName(p.track)}</td>}
                        <td>{p.measure}</td>
                        <td>
                          <button
                            onClick={() => {
                              void api
                                .command({ type: "stopFingerDemo" })
                                .catch((error) => setError(String(error)));
                              setPreview(
                                result.context.findIndex(
                                  (n) =>
                                    n.track === p.track && n.index === p.index,
                                ),
                              );
                            }}
                          >
                            {pitchName(p.pitch)}
                          </button>
                        </td>
                        <td>{p.saved ?? "—"}</td>
                        <td>
                          <select
                            disabled={busy}
                            aria-label={`${noteLabel(p)} 建议手指`}
                            data-finger-position={positionKey(p)}
                            value={values[positionKey(p)] ?? p.finger}
                            onChange={(e) =>
                              setValues((v) => ({
                                ...v,
                                [positionKey(p)]: Number(e.target.value),
                              }))
                            }
                          >
                            {[1, 2, 3, 4, 5].map((f) => (
                              <option value={f} key={f}>
                                {f}
                              </option>
                            ))}
                          </select>
                        </td>
                        <td>{p.confidence}%</td>
                        <td>{p.reason}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              {!result.proposals.length && (
                <p>该选段没有可安全推荐的指法。可尝试缩小选段或手动标注。</p>
              )}
            </div>
            <div className="fingering-pages">
              <button
                aria-label="上一页指法"
                disabled={!page}
                onClick={() => setPage(page - 1)}
              >
                <ChevronLeft size={16} />
              </button>
              <span>
                {page + 1} /{" "}
                {Math.max(1, Math.ceil(result.proposals.length / 40))}
              </span>
              <button
                aria-label="下一页指法"
                disabled={(page + 1) * 40 >= result.proposals.length}
                onClick={() => setPage(page + 1)}
              >
                <ChevronRight size={16} />
              </button>
            </div>
          </>
        ) : (
          <div className="fingering-empty">
            选好音轨、手的跨度与范围，然后生成建议。生成过程不会修改已有指法。
          </div>
        )}
        <div className="dialog-actions">
          <button
            disabled={busy}
            onClick={() =>
              void perform(async () => {
                await api.command({
                  type: "undoFingersFor",
                  content_id: song.contentId,
                });
                await changed();
                setResult(null);
              })
            }
          >
            撤销上次指法修改
          </button>
          <span className="menu-spacer" />
          <button
            onClick={() => {
              if (!busy) close();
            }}
          >
            取消
          </button>
          <button
            className="primary-button"
            disabled={
              busy ||
              !result ||
              !accepted.size ||
              !currentAcceptance?.valid ||
              ((currentAcceptance.selected < currentAcceptance.total ||
                currentAcceptance.contextUnmarked > 0) &&
                !acknowledged) ||
              Object.keys(values).length > 0
            }
            onClick={() => void apply()}
          >
            接受并保存 {accepted.size || ""}
          </button>
        </div>
      </section>
    </div>
  );
}

function FingerPosition({
  result,
  plan,
  preview,
  setPreview,
  hand,
  disabled,
  previewFingers,
}: {
  result: Result;
  plan: Plan;
  preview: number;
  setPreview: (n: number) => void;
  hand: string;
  disabled: boolean;
  previewFingers?: (number | null)[];
}) {
  const [demoTime, setDemoTime] = useState<number | null>(null);
  const [demonstrating, setDemonstrating] = useState(false);
  const context = result.context;
  if (!plan || !context.length) return null;
  const selected = context[Math.min(preview, context.length - 1)];
  const onset = demoTime ?? selected.onset;
  const active = context
    .map((n, i) => ({
      ...n,
      finger: previewFingers ? previewFingers[i] : plan.contextFingers[i],
    }))
    .filter(
      (n) =>
        ((n.onset <= onset && n.end > onset) || n.onset === onset) &&
        (demoTime === null ||
          n.target ||
          n.track !== result.request.track ||
          n.onset < (context.find((position) => position.target)?.onset ?? 0)),
    );
  const firstTarget = context.find((n) => n.target)?.onset ?? selected.onset;
  const keyboardNotes =
    demoTime !== null
      ? context.filter(
          (n) =>
            n.target ||
            n.track !== result.request.track ||
            (n.onset < firstTarget && n.end > firstTarget),
        )
      : active.length
        ? active
        : [selected];
  const low = Math.max(
      21,
      Math.floor(Math.min(...keyboardNotes.map((n) => n.pitch)) / 12) * 12,
    ),
    high = Math.min(
      108,
      Math.max(
        low + 12,
        Math.ceil(Math.max(...keyboardNotes.map((n) => n.pitch)) / 12) * 12,
      ),
    );
  const black = (p: number) => [1, 3, 6, 8, 10].includes(p % 12);
  const whites = Array.from(
    { length: high - low + 1 },
    (_, i) => low + i,
  ).filter((p) => !black(p));
  const width = 36;
  const x = (p: number) =>
    black(p)
      ? whites.filter((w) => w < p).length * width - 11
      : whites.indexOf(p) * width;
  const groups = context
    .map((n, i) => i)
    .filter((i) => i === 0 || context[i].onset !== context[i - 1].onset);
  const at = groups.findIndex((i) => context[i].onset === selected.onset);
  return (
    <div className="finger-position">
      <div>
        <strong>
          {previewFingers ? "保存结果手位" : "推荐方案手位"} ·{" "}
          {hand === "left" ? "左手" : "右手"} · 第 {selected.measure} 小节
        </strong>
        <button
          disabled={demonstrating || at <= 0}
          onClick={() => {
            setDemoTime(null);
            setPreview(groups[at - 1]);
          }}
        >
          上一个位置
        </button>
        <span>
          {at + 1} / {groups.length}
        </span>
        <button
          disabled={demonstrating || at >= groups.length - 1}
          onClick={() => {
            setDemoTime(null);
            setPreview(groups[at + 1]);
          }}
        >
          下一个位置
        </button>
      </div>
      <FingerDemoControls
        result={result}
        plan={plan}
        time={setDemoTime}
        active={setDemonstrating}
        move={setPreview}
        disabled={disabled || !!previewFingers}
      />
      <svg
        viewBox={`-2 -2 ${Math.max(width * whites.length, 72) + 4} 116`}
        role="img"
        aria-label="当前和弦及仍按住的音符指法"
      >
        {[false, true].flatMap((b) =>
          Array.from({ length: high - low + 1 }, (_, i) => low + i)
            .filter((p) => black(p) === b)
            .map((p) => {
              const keyNotes = active.filter((n) => n.pitch === p);
              const n = keyNotes[0];
              const label = [
                ...new Set(keyNotes.map((n) => n.finger ?? "·")),
              ].join("/");
              return (
                <g key={p}>
                  <rect
                    x={x(p)}
                    y={0}
                    width={b ? 22 : width}
                    height={b ? 68 : 104}
                    rx={2}
                    fill={
                      n
                        ? hand === "left"
                          ? "#639bec"
                          : "#69c5a3"
                        : b
                          ? "#252a32"
                          : "#dedfdc"
                    }
                    stroke="#171b21"
                    strokeWidth={1}
                  />
                  {n && (
                    <text
                      x={x(p) + (b ? 11 : 18)}
                      y={b ? 48 : 86}
                      textAnchor="middle"
                      fill="#111820"
                      fontSize={label.length > 1 ? 12 : 17}
                      fontWeight={700}
                    >
                      {label}
                    </text>
                  )}
                </g>
              );
            }),
        )}
      </svg>
      <small>
        键上数字为手指
        {previewFingers
          ? "，· 表示仍未标记；当前显示保存后的实际组合。切回完整推荐方案可示范声音。"
          : "；当前显示完整推荐方案。"}
        显示本次发音和仍在按住的音符。
      </small>
    </div>
  );
}
