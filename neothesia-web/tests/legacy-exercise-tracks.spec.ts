import { expect, test } from "@playwright/test";

test("单左手练习可核对旧资料，缺少旧原件时说明原因", async ({ page, request }) => {
  const response = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: { type: "generate", spec: { tonic: 2, tonality: "Major", minor_form: "Natural", pattern: "Scale", direction: "UpAndDown", hands: "Left", octaves: 1, repetitions: 1, tempo_bpm: 120 } },
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  await page.goto("/");
  await page.getByRole("button", { name: "核对旧版左手资料" }).click();
  const dialog = page.getByRole("dialog", { name: "旧版左手练习资料核对" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("alert")).toContainText("没有找到保留的旧版单左手 MIDI 原件");
  await dialog.getByRole("button", { name: "关闭旧版资料核对" }).click();
  await expect(dialog).toHaveCount(0);
});
