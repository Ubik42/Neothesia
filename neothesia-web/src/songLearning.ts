export const learningStages: Record<string, string> = {
  planned: "待学",
  learning: "学习中",
  polishing: "精练中",
  maintaining: "巩固复习",
  mastered: "已掌握",
  paused: "暂停",
};
export type LearningDraft = {
  stage: string;
  goal: string;
  due: string | null;
  weeklyMinutes: number | null;
  targetBpm: number | null;
};
export type SongLearning = LearningDraft & {
  updatedAt: number;
  title: string;
  transitions: { stage: string; at: number }[];
};
export const localDay = (date = new Date()) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
export const activeDeadline = (plan: SongLearning | undefined) =>
  !!plan?.due && !["mastered", "paused"].includes(plan.stage);
export function learningMatches(
  plan: SongLearning | undefined,
  filter: string,
  today: string,
) {
  if (filter === "all") return true;
  if (filter === "untracked") return !plan;
  if (filter === "overdue") return activeDeadline(plan) && plan!.due! < today;
  if (filter === "today") return activeDeadline(plan) && plan!.due === today;
  return plan?.stage === filter;
}
export function compareLearning(
  a: SongLearning | undefined,
  b: SongLearning | undefined,
  sort: string,
  direction: string,
) {
  const stages = Object.keys(learningStages);
  const x =
    sort === "learningStage"
      ? a
        ? stages.indexOf(a.stage)
        : null
      : (a?.due ?? null);
  const y =
    sort === "learningStage"
      ? b
        ? stages.indexOf(b.stage)
        : null
      : (b?.due ?? null);
  if (x === null || y === null) return x === y ? 0 : x === null ? 1 : -1;
  const value =
    typeof x === "number" && typeof y === "number"
      ? x - y
      : String(x).localeCompare(String(y));
  return direction === "desc" ? -value : value;
}
