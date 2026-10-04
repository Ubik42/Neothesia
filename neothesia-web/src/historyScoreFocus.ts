export type HistoryMeasureSpan = {
  measure: number;
  occurrence: number;
  at: number;
  end: number;
  songStart: number;
  songEnd: number;
  measureStart: number;
  measureEnd: number;
};
export type HistoryScoreTarget = {
  contentId: string;
  scoreRevision: number;
  historyId: string;
  reference: number;
  track: number;
  index: number;
  pitch: number;
  measure: number;
  kind: string;
};
export type HistoryScoreFocus = HistoryScoreTarget & { request: string };

export type HistoryReviewNote = {
  sourceTitle?: string;
  sourceContentId?: string;
  sourceHistoryId?: string;
  sourceReference?: number;
  reference: number;
  track: number;
  index: number | null;
  pitch: number;
  measure: number;
  songTime: number;
  at: number;
  end: number;
  kind: string;
  offsetMs: number | null;
  referenceVelocity: number;
  actualVelocity: number | null;
};
export type HistoryScoreReview = {
  contentId: string;
  historyId: string;
  scoreRevision: number;
  recordedAt: number;
  title: string;
  truncated: boolean;
  exact: boolean;
  wrongNotes: number | null;
  notes: HistoryReviewNote[];
  measures?: HistoryMeasureSpan[] | null;
};
export const historyGradeName = (kind: string) =>
  (
    ({
      onTime: "准时",
      early: "偏早",
      late: "偏晚",
      missed: "漏音",
      pending: "未计分",
      approximate: "近似对照",
    }) as Record<string, string>
  )[kind] ?? kind;
