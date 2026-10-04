use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub mode: String,
    pub value: String,
}
pub type Rules = BTreeMap<String, Rule>;
#[derive(Serialize, Deserialize)]
struct Store {
    version: u8,
    revision: u64,
    templates: BTreeMap<String, Rules>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            templates: BTreeMap::new(),
        }
    }
}
pub fn validate(rules: &Rules) -> Result<(), String> {
    if rules.is_empty() || rules.len() > 6 {
        return Err("请选择资料模板字段".into());
    }
    let mut changed = false;
    for (field, rule) in rules {
        let modes = match field.as_str() {
            "composer" | "artist" | "collection" | "difficulty" => &["keep", "set", "clear"][..],
            "notes" => &["keep", "set", "append", "clear"][..],
            "tags" => &["keep", "replace", "append", "remove", "clear"][..],
            _ => return Err("资料模板字段无效".into()),
        };
        if !modes.contains(&rule.mode.as_str()) {
            return Err("资料模板操作无效".into());
        }
        let limit = if field == "notes" {
            10000
        } else if field == "tags" {
            12900
        } else {
            2000
        };
        if rule.value.chars().count() > limit {
            return Err("资料模板字段过长".into());
        }
        if !["keep", "clear"].contains(&rule.mode.as_str()) && rule.value.trim().is_empty() {
            return Err("请填写修改内容，或明确选择清空".into());
        }
        if field == "tags" {
            let tags = rule
                .value
                .split([',', '，', ';', '；', '\n'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if tags.len() > 100 || tags.iter().any(|s| s.chars().count() > 128) {
                return Err("最多 100 个标签，每个最多 128 字".into());
            }
        }
        changed |= rule.mode != "keep";
    }
    if !changed {
        return Err("模板至少要修改一个字段".into());
    }
    Ok(())
}
fn read(data: &Path) -> Result<Store, String> {
    match std::fs::read(data.join("metadata-templates.json")) {
        Ok(bytes) => {
            let value: Store =
                serde_json::from_slice(&bytes).map_err(|e| format!("资料模板无法读取：{e}"))?;
            if value.version != 1
                || value.templates.len() > 50
                || value.templates.iter().any(|(name, rules)| {
                    name.trim().is_empty() || name.chars().count() > 80 || validate(rules).is_err()
                })
            {
                return Err("资料模板记录无效".into());
            }
            Ok(value)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Store::default()),
        Err(e) => Err(e.to_string()),
    }
}
fn view(value: &Store) -> serde_json::Value {
    serde_json::json!({"revision":value.revision,"templates":value.templates.iter().map(|(name,rules)|serde_json::json!({"name":name,"rules":rules})).collect::<Vec<_>>()})
}
pub fn inspect(data: &Path) -> Result<serde_json::Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "资料模板正忙")?;
    read(data).map(|value| view(&value))
}
pub fn update(
    data: &Path,
    name: String,
    rules: Option<Rules>,
    expected: u64,
) -> Result<serde_json::Value, String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err("模板名称需为 1 到 80 字".into());
    }
    if let Some(rules) = &rules {
        validate(rules)?;
    }
    let lock = crate::library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "资料模板正忙")?;
    let mut value = read(data)?;
    if value.revision != expected {
        return Err("资料模板已变化，请刷新模板列表后再保存；当前字段设置保留".into());
    }
    if let Some(rules) = rules {
        if !value.templates.contains_key(&name) && value.templates.len() >= 50 {
            return Err("最多保存 50 套资料模板".into());
        }
        value.templates.insert(name, rules);
    } else {
        value.templates.remove(&name);
    }
    value.revision = value.revision.checked_add(1).ok_or("模板版本超出范围")?;
    std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let tmp = data.join("metadata-templates.json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, data.join("metadata-templates.json")).map_err(|e| e.to_string())?;
    Ok(view(&value))
}

pub fn apply(
    mut base: neothesia_core::library::SongMetadata,
    rules: &Rules,
) -> Result<neothesia_core::library::SongMetadata, String> {
    validate(rules)?;
    for (field, rule) in rules {
        let text = rule.value.trim();
        if rule.mode == "keep" {
            continue;
        }
        if field == "tags" {
            let tags = text
                .split([',', '，', ';', '；', '\n'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect::<Vec<_>>();
            match rule.mode.as_str() {
                "clear" => base.tags.clear(),
                "replace" => base.tags = tags,
                "append" => base.tags.extend(tags),
                "remove" => base
                    .tags
                    .retain(|t| !tags.iter().any(|tag| tag.eq_ignore_ascii_case(t))),
                _ => unreachable!(),
            }
            continue;
        }
        let target = match field.as_str() {
            "composer" => &mut base.composer,
            "artist" => &mut base.artist,
            "collection" => &mut base.collection,
            "difficulty" => &mut base.difficulty,
            "notes" => &mut base.notes,
            _ => unreachable!(),
        };
        if rule.mode == "clear" {
            *target = None;
        } else if rule.mode == "append" {
            if let Some(current) = target {
                if current != text && !current.ends_with(&format!("\n{text}")) {
                    current.push('\n');
                    current.push_str(text);
                }
            } else {
                *target = Some(text.into());
            }
        } else {
            *target = Some(text.into());
        }
    }
    let base = base.normalized();
    if base
        .notes
        .as_ref()
        .is_some_and(|s| s.chars().count() > 10000)
        || base.tags.len() > 100
    {
        return Err("合并后备注或标签超过容量，请缩短内容或调整操作".into());
    }
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn template_registry_conflicts_normalized_preview_and_capacity() {
        let data = std::env::temp_dir().join(format!(
            "neothesia-templates-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let rules:Rules=serde_json::from_value(serde_json::json!({"tags":{"mode":"append","value":"NEW,new,学生"},"notes":{"mode":"append","value":"老师提示"}})).unwrap();
        let mut base = neothesia_core::library::SongMetadata::default();
        base.title = Some("原曲名".into());
        base.notes = Some("原备注".into());
        base.composer = Some("原作".into());
        base.tags = vec!["保留".into()];
        let value = apply(base.clone(), &rules).unwrap();
        assert_eq!(value.title, base.title);
        assert_eq!(value.composer, base.composer);
        assert_eq!(value.notes.as_deref(), Some("原备注\n老师提示"));
        assert_eq!(
            value
                .tags
                .iter()
                .filter(|t| t.eq_ignore_ascii_case("new"))
                .count(),
            1
        );
        assert_eq!(apply(value.clone(), &rules).unwrap(), value);
        base.notes = Some("长".repeat(10000));
        assert!(apply(base, &rules).is_err());
        let initial = inspect(&data).unwrap();
        assert_eq!(initial["revision"], 0);
        let saved = update(&data, "课堂".into(), Some(rules.clone()), 0).unwrap();
        assert_eq!(saved["revision"], 1);
        let path = data.join("metadata-templates.json");
        let bytes = std::fs::read(&path).unwrap();
        assert!(update(&data, "课堂".into(), None, 0).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(inspect(&data).unwrap()["templates"][0]["name"], "课堂");
        update(&data, "课堂".into(), None, 1).unwrap();
        std::fs::write(&path, b"broken").unwrap();
        assert!(update(&data, "课堂".into(), Some(rules), 2).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"broken");
    }
}

pub fn snapshot_unlocked(data:&Path)->Result<BTreeMap<String,Rules>,String>{Ok(read(data)?.templates)}
pub fn merge_bytes_unlocked(data:&Path,incoming:&BTreeMap<String,Rules>,replace:bool)->Result<Option<Vec<u8>>,String>{let mut value=read(data)?;let before=value.templates.clone();for(name,rules)in incoming{if name!=name.trim()||name.trim().is_empty()||name.chars().count()>80||name.chars().any(char::is_control){return Err("备份模板名称无效".into());}validate(rules)?;if replace||!value.templates.contains_key(name){value.templates.insert(name.clone(),rules.clone());}}if value.templates.len()>50{return Err("合并后资料模板超过 50 套，请先整理本地模板".into());}if value.templates==before{return Ok(None);}value.revision=value.revision.checked_add(1).ok_or("模板版本超出范围")?;Ok(Some(serde_json::to_vec_pretty(&value).map_err(|e|e.to_string())?))}
