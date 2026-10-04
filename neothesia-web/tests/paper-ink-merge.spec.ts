import { test, expect } from "@playwright/test";
import { reviewInkMerge, applyInkMerge } from "../src/paperInkMerge";
import type { PaperAnnotation } from "../src/PaperAnnotations";
const mark = (
  id: string,
  patch: Partial<PaperAnnotation> = {},
): PaperAnnotation => ({
  id,
  assetId: "page",
  page: 1,
  x: 0.1,
  y: 0.2,
  width: 0.3,
  height: 0.2,
  text: "",
  color: "red",
  ink: {
    points: [
      [0, 0],
      [1, 1],
    ],
    thickness: 0.002,
  },
  ...patch,
});
test("三方笔迹合并保留独立更改、删除、新增与明确冲突选择", () => {
  const original = [
      mark("a"),
      mark("b"),
      mark("c"),
      mark("d"),
      mark("restore"),
    ],
    draft = [
      mark("a"),
      mark("b", { text: "本机要求" }),
      mark("c"),
      mark("restore", { text: "继续保留" }),
      mark("new-local"),
    ],
    current = [
      mark("a", { color: "green" }),
      mark("b", { color: "blue" }),
      mark("d"),
      mark("new-saved"),
    ];
  const review = reviewInkMerge(original, draft, current);
  expect(review.conflicts.map((n) => n.id)).toEqual(["b", "restore"]);
  const defaults = applyInkMerge(review, {});
  expect(defaults.map((n) => n.id)).toEqual([
    "a",
    "b",
    "new-saved",
    "new-local",
  ]);
  expect(defaults[0].color).toBe("green");
  expect(defaults[1].color).toBe("blue");
  const selected = applyInkMerge(review, { b: "local", restore: "local" });
  expect(selected.find((n) => n.id === "b")?.text).toBe("本机要求");
  expect(selected.find((n) => n.id === "restore")?.text).toBe("继续保留");
  expect(selected.some((n) => n.id === "c" || n.id === "d")).toBeFalsy();
  expect(current[1].color).toBe("blue");
});
