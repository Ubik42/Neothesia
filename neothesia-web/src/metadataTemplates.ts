export const templateFields = [
  ["composer", "作曲家"],
  ["artist", "演奏者"],
  ["collection", "作品集"],
  ["difficulty", "难度"],
  ["tags", "标签"],
  ["notes", "备注"],
] as const;
export type TemplateField = (typeof templateFields)[number][0];
export type Rule = { mode: string; value: string };
export type Rules = Record<TemplateField, Rule>;
export const emptyRules = (): Rules =>
  Object.fromEntries(
    templateFields.map(([key]) => [key, { mode: "keep", value: "" }]),
  ) as Rules;
export const modes = (field: TemplateField) =>
  field === "tags"
    ? [
        ["keep", "保留"],
        ["append", "添加标签"],
        ["remove", "移除指定标签"],
        ["replace", "替换全部标签"],
        ["clear", "清空"],
      ]
    : field === "notes"
      ? [
          ["keep", "保留"],
          ["append", "追加备注"],
          ["set", "替换"],
          ["clear", "清空"],
        ]
      : [
          ["keep", "保留"],
          ["set", "替换"],
          ["clear", "清空"],
        ];
export function rulesError(rules: Rules): string {
  if (!templateFields.some(([field]) => rules[field].mode !== "keep"))
    return "请至少选择一个要修改的字段。";
  for (const [field, label] of templateFields) {
    const r = rules[field];
    if (!modes(field).some(([m]) => m === r.mode)) return `${label}操作无效。`;
    if (!["keep", "clear"].includes(r.mode) && !r.value.trim())
      return `请填写${label}，或明确选择清空。`;
    if (
      r.value.length >
      (field === "notes" ? 10000 : field === "tags" ? 12900 : 2000)
    )
      return `${label}内容过长。`;
  }
  return "";
}
export const metadataText = (value: unknown) =>
  Array.isArray(value) ? value.join("，") : String(value ?? "（空）");
