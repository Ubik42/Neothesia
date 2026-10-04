import { useEffect, useRef, useState } from "react";
import { PracticeTimePanel } from "./PracticeTimePanel";
import { X } from "lucide-react";
import { api } from "./api";
import {
  learningStages,
  type SongLearning,
  type LearningDraft,
} from "./songLearning";
export function SongLearningDialog({
  path,
  contentId,
  title,
  previous,
  close,
  changed,
}: {
  path: string;
  contentId: string;
  title: string;
  previous: SongLearning | null;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const initial = useRef<LearningDraft>(
    previous
      ? {
          stage: previous.stage,
          goal: previous.goal,
          due: previous.due,
          weeklyMinutes: previous.weeklyMinutes,
          targetBpm: previous.targetBpm,
        }
      : {
          stage: "planned",
          goal: "",
          due: null,
          weeklyMinutes: null,
          targetBpm: null,
        },
  );
  const expected = useRef(previous);
  const [draft, setDraft] = useState(initial.current),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [discard, setDiscard] = useState(false);
  const working = useRef(false);
  const dirty = JSON.stringify(draft) !== JSON.stringify(initial.current);
  const attemptClose = () => {
    if (!working.current) {
      if (dirty) setDiscard(true);
      else close();
    }
  };
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopImmediatePropagation();
        event.preventDefault();
        attemptClose();
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [dirty, close]);
  const save = async (value: LearningDraft | null) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      await api.command({
        type: "saveSongLearning",
        path,
        content_id: contentId,
        value,
        expected: expected.current,
      });
      await changed();
      close();
    } catch (error) {
      setError(String(error));
    } finally {
      working.current = false;
      setBusy(false);
    }
  };
  return (
    <div
      className="modal-backdrop song-learning-backdrop"
      onClick={(event) => {
        event.stopPropagation();
        attemptClose();
      }}
    >
      <section
        className="settings-dialog song-learning-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="song-learning-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="song-learning-title">曲目学习</h2>
            <p>{title}</p>
          </div>
          <button
            autoFocus
            aria-label="关闭曲目学习"
            disabled={busy}
            onClick={attemptClose}
          >
            <X size={18} />
          </button>
        </div>
        <p className="parameter-help">
          按这首曲目的内容保存，同内容的副本共用资料。学习阶段由你确认，目标速度和计划时长不代表已经达标。
        </p>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void save(draft);
          }}
        >
          <div className="song-learning-fields">
            <label>
              学习阶段
              <select
                aria-label="曲目学习阶段"
                disabled={busy}
                value={draft.stage}
                onChange={(event) =>
                  setDraft({ ...draft, stage: event.target.value })
                }
              >
                {Object.entries(learningStages).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              目标日期
              <input
                type="date"
                aria-label="曲目目标日期"
                disabled={busy}
                value={draft.due ?? ""}
                onChange={(event) =>
                  setDraft({ ...draft, due: event.target.value || null })
                }
              />
            </label>
            <label>
              每周计划 · 分钟
              <input
                type="number"
                min={1}
                max={10080}
                step={1}
                aria-label="曲目每周计划分钟"
                placeholder="未设置"
                disabled={busy}
                value={draft.weeklyMinutes ?? ""}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    weeklyMinutes:
                      event.target.value === ""
                        ? null
                        : Number(event.target.value),
                  })
                }
              />
            </label>
            <label>
              目标速度 · BPM
              <input
                type="number"
                min={20}
                max={300}
                step={1}
                aria-label="曲目目标速度"
                placeholder="未设置"
                disabled={busy}
                value={draft.targetBpm ?? ""}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    targetBpm:
                      event.target.value === ""
                        ? null
                        : Number(event.target.value),
                  })
                }
              />
            </label>
          </div>
          <PracticeTimePanel
            contentId={contentId}
            targetMinutes={draft.weeklyMinutes}
          />
          <label className="song-learning-goal">
            目标与达标要求
            <textarea
              aria-label="曲目学习目标"
              maxLength={2000}
              rows={4}
              disabled={busy}
              value={draft.goal}
              onChange={(event) =>
                setDraft({ ...draft, goal: event.target.value })
              }
            />
          </label>
          {error && (
            <p className="dialog-error" role="alert">
              {error}
            </p>
          )}
          {discard && (
            <div className="song-learning-discard">
              <span>有未保存修改。</span>
              <button type="button" disabled={busy} onClick={close}>
                放弃修改
              </button>
              <button type="button" onClick={() => setDiscard(false)}>
                继续编辑
              </button>
            </div>
          )}
          <div className="dialog-actions">
            <button type="button" disabled={busy} onClick={attemptClose}>
              取消
            </button>
            {previous && (
              <button
                type="button"
                disabled={busy}
                onClick={() => void save(null)}
              >
                移除学习目标
              </button>
            )}
            <button type="submit" className="primary-button" disabled={busy}>
              {busy ? "正在保存…" : "保存学习资料"}
            </button>
          </div>
        </form>
        {!!expected.current?.transitions.length && (
          <details className="song-learning-history">
            <summary>
              阶段变更 · {expected.current.transitions.length} 次
            </summary>
            <ol>
              {[...expected.current.transitions]
                .reverse()
                .map((transition, index) => (
                  <li key={index}>
                    <time>
                      {new Date(transition.at * 1000).toLocaleString("zh-CN")}
                    </time>
                    <strong>
                      {learningStages[transition.stage] ?? transition.stage}
                    </strong>
                  </li>
                ))}
            </ol>
          </details>
        )}
      </section>
    </div>
  );
}
