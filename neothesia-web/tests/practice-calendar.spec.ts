import { test, expect } from "@playwright/test";

test.use({ timezoneId: "America/Denver" });
test("运行区间按本地午夜拆分，夏令时日期和比例用时保持总和", async ({
  page,
}) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const path = "/src/practiceTime.ts";
    const { practiceDayRange, practiceIntervalDays } = await import(
      /* @vite-ignore */ path
    );
    const spring = practiceDayRange(new Date(2026, 2, 8));
    const autumn = practiceDayRange(new Date(2026, 10, 1));
    const start = new Date(2026, 9, 10, 23, 59, 0).getTime(),
      end = new Date(2026, 9, 11, 0, 1, 0).getTime();
    const buckets = [
      ...practiceIntervalDays([{ start, end, runningMs: 90000 }], start, end),
    ];
    return {
      spring: spring.to - spring.from,
      autumn: autumn.to - autumn.from,
      buckets,
    };
  });
  expect(result.spring).toBe(23 * 60 * 60 * 1000);
  expect(result.autumn).toBe(25 * 60 * 60 * 1000);
  expect(result.buckets).toEqual([
    ["2026-10-10", 45000],
    ["2026-10-11", 45000],
  ]);
});
