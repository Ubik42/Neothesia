import { test, expect } from "@playwright/test";
test("完整备份逐块校验并保留文件及移机核对状态", async ({ page }) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const path = "/src/workspaceBackup.ts";
    const { encodeWorkspaceBackup: encode, decodeWorkspaceBackup: decode } =
      await import(path);
    const content = new Uint8Array(1_048_607);
    content[0] = 17;
    content[content.length - 1] = 83;
    const batch = (id: string) => ({
      id,
      name: id,
      revision: 2,
      updated: 12,
      prepared: true,
      books: [["old", "book"]],
      midiTargets: [],
      items: [
        {
          id: "two",
          file: new File(["page two"], "第二页.png", {
            type: "image/png",
            lastModified: 1234,
          }),
          kind: "image",
          selected: true,
          manual: false,
          group: "页",
          state: "done",
          message: "",
        },
        {
          id: "one",
          file: new File(["page one"], "第一页.png", {
            type: "image/png",
            lastModified: 4321,
          }),
          kind: "image",
          selected: true,
          manual: false,
          group: "页",
          state: "pending",
          message: "",
        },
      ],
    });
    const blob = await encode(
        new Blob([content]),
        [batch("第一批"), batch("第二批")],
        () => {},
      ),
      value = await decode(blob, () => {}),
      again = await decode(blob, () => {});
    const header = new Uint8Array(await blob.slice(0, 12).arrayBuffer()),
      length = new DataView(header.buffer).getUint32(8, true),
      meta = JSON.parse(await blob.slice(12, 12 + length).text());
    const reject = async (file: Blob) => {
      try {
        await decode(file, () => {});
        return false;
      } catch {
        return true;
      }
    };
    const change = (edit: (m: typeof meta) => void) => {
      const m = structuredClone(meta);
      edit(m);
      const b = new TextEncoder().encode(JSON.stringify(m)),
        h = header.slice();
      new DataView(h.buffer).setUint32(8, b.length, true);
      return new Blob([h, b, blob.slice(12 + length)]);
    };
    const at = 12 + length + 1_048_576,
      badHeader = header.slice();
    new DataView(badHeader.buffer).setUint32(8, 2_000_001, true);
    return {
      practice: Array.from(
        new Uint8Array(await value.practice.arrayBuffer()).filter(
          (v) => v !== 0,
        ),
      ),
      names: value.batches.map((b: { name: string }) => b.name),
      stable: value.batches[0].id === again.batches[0].id,
      files: await Promise.all(
        value.batches[0].items.map(
          async (i: { file: File; state: string; selected: boolean }) => [
            i.file.name,
            await i.file.text(),
            i.file.type,
            i.file.lastModified,
            i.state,
            i.selected,
          ],
        ),
      ),
      portable: [
        value.batches[0].revision,
        value.batches[0].prepared,
        value.batches[0].books.length,
      ],
      rejected: await Promise.all([
        reject(
          new Blob([
            blob.slice(0, at),
            new Uint8Array([255]),
            blob.slice(at + 1),
          ]),
        ),
        reject(blob.slice(0, blob.size - 1)),
        reject(new Blob([blob, "extra"])),
        reject(new Blob([badHeader, blob.slice(12)])),
        reject(
          change((m) => {
            m.practice.name = "../evil";
          }),
        ),
        reject(
          change((m) => {
            m.practice.size = 256_000_001;
          }),
        ),
        reject(
          change((m) => {
            m.batches = Array(201).fill(m.batches[0]);
          }),
        ),
        reject(
          change((m) => {
            m.batches[1].name = m.batches[0].name;
          }),
        ),
      ]),
    };
  });
  expect(result.practice).toEqual([17, 83]);
  expect(result.names).toEqual(["第一批", "第二批"]);
  expect(result.stable).toBe(true);
  expect(result.files).toEqual([
    ["第二页.png", "page two", "image/png", 1234, "review", false],
    ["第一页.png", "page one", "image/png", 4321, "pending", false],
  ]);
  expect(result.portable).toEqual([0, false, 0]);
  expect(result.rejected).toEqual(Array(8).fill(true));
});
test("批次恢复以真实IndexedDB整笔提交或回滚", async ({ page }) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const path = "/src/batchImportStore.ts",
      {
        saveImportedBatches: save,
        loadBatch,
        listBatches,
      } = await import(path);
    const batch = (id: string) => ({
      id,
      name: id,
      revision: 0,
      updated: 0,
      prepared: false,
      books: [],
      midiTargets: [],
      items: [
        {
          id: "two",
          file: new File(["two"], "02.png", {
            type: "image/png",
            lastModified: 20,
          }),
          kind: "image",
          selected: false,
          manual: false,
          group: "谱",
          state: "review",
          message: "",
        },
        {
          id: "one",
          file: new File(["one"], "01.png", {
            type: "image/png",
            lastModified: 10,
          }),
          kind: "image",
          selected: false,
          manual: false,
          group: "谱",
          state: "pending",
          message: "",
        },
      ],
    });
    await save([batch("existing-a"), batch("existing-b")]);
    let conflict = "",
      quota = "",
      duplicate = "";
    try {
      await save([batch("rollback"), batch("existing-a")]);
    } catch (e) {
      conflict = String(e);
    }
    try {
      await save([batch("duplicate"), batch("duplicate")]);
    } catch (e) {
      duplicate = String(e);
    }
    const add = IDBObjectStore.prototype.add;
    let files = 0;
    IDBObjectStore.prototype.add = function (
      ...args: Parameters<IDBObjectStore["add"]>
    ) {
      if (this.name === "files" && ++files === 3)
        throw new DOMException("disk quota", "QuotaExceededError");
      return add.apply(this, args);
    };
    try {
      await save([batch("quota-one"), batch("quota-two")]);
    } catch (e) {
      quota = String(e);
    } finally {
      IDBObjectStore.prototype.add = add;
    }
    const saved = await loadBatch("existing-a"),
      summaries = await listBatches();
    const keys = await new Promise<IDBValidKey[]>((resolve, reject) => {
      const r = indexedDB.open("neothesia-score-imports", 1);
      r.onsuccess = () => {
        const db = r.result,
          tx = db.transaction("files"),
          query = tx.objectStore("files").getAllKeys();
        query.onsuccess = () => resolve(query.result);
        tx.oncomplete = () => db.close();
      };
      r.onerror = () => reject(r.error);
    });
    return {
      conflict,
      quota,
      duplicate,
      ids: summaries.map((b: { id: string }) => b.id).sort(),
      keys,
      files: await Promise.all(
        saved.items.map(async (i: { file: File }) => [
          i.file.name,
          await i.file.text(),
          i.file.type,
          i.file.lastModified,
        ]),
      ),
      revision: saved.revision,
    };
  });
  expect(result.conflict).toContain("已恢复");
  expect(result.quota).toContain("存储空间不足");
  expect(result.duplicate).toContain("重复");
  expect(result.ids).toEqual(["existing-a", "existing-b"]);
  expect(result.keys).toHaveLength(4);
  expect(
    result.keys.every(
      (key) => Array.isArray(key) && String(key[0]).startsWith("existing-"),
    ),
  ).toBe(true);
  expect(result.files).toEqual([
    ["02.png", "two", "image/png", 20],
    ["01.png", "one", "image/png", 10],
  ]);
  expect(result.revision).toBe(1);
});
