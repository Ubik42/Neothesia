use crate::{
    library,
    library_workspace::{Entry, Workspace, key, lock_for},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkRecord {
    pub content_id: String,
    pub from: PathBuf,
    pub to: PathBuf,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkItem {
    pub content_id: String,
    pub from: PathBuf,
    pub to: PathBuf,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplacementReview {
    pub old_content_id: String,
    pub new_content_id: String,
    pub action: String,
    pub target: Option<PathBuf>,
}

fn replacement_candidates(w: &Workspace, old_id: &str, replaced: &Path) -> Vec<PathBuf> {
    w.entries.values()
        .filter(|e| e.content_id.as_deref() == Some(old_id) && key(&e.path) != key(replaced))
        .filter(|e| e.path.is_file())
        .filter(|e| midi_file::MidiFile::new(&e.path).is_ok_and(|f| f.content_id == old_id))
        .map(|e| e.path.clone())
        .collect()
}

pub fn replacements(data: &Path, query: &str, include_reviewed: bool, offset: usize, limit: usize) -> Result<Value, String> {
    let lock = lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件处理信息正忙")?;
    let w = Workspace::load(data)?;
    let mut rows = Vec::new();
    for (record, old) in &w.retired_entries {
        let Some(old_id) = old.content_id.as_ref() else { continue };
        let Some(current) = w.entries.get(&key(&old.path)) else { continue };
        let Some(new_id) = current.content_id.as_ref() else { continue };
        if old_id == new_id { continue; }
        let review = w.replacement_reviews.get(record).filter(|r| &r.new_content_id == new_id);
        if review.is_some() && !include_reviewed { continue; }
        let title = old.metadata.title.clone().filter(|s| !s.is_empty()).unwrap_or_else(||old.path.file_stem().unwrap_or_default().to_string_lossy().into());
        let haystack = format!("{title} {} {old_id} {new_id}",old.path.display()).to_lowercase();
        if !query.to_lowercase().split_whitespace().all(|t|haystack.contains(t)) { continue; }
        let candidates = replacement_candidates(&w,old_id,&old.path);
        rows.push(json!({"path":old.path,"title":title,"oldContentId":old_id,"newContentId":new_id,"oldNotes":old.notes,"newNotes":current.notes,"oldDuration":old.duration,"newDuration":current.duration,"oldTracks":old.tracks,"newTracks":current.tracks,"candidates":candidates,"review":review,"reviewed":review.is_some(),"available":current.available,"error":current.error}));
    }
    rows.sort_by(|a,b|a["title"].as_str().cmp(&b["title"].as_str()).then(a["path"].as_str().cmp(&b["path"].as_str())));
    let total=rows.len();
    let limit=limit.clamp(1,100);
    let offset=offset.min(total.saturating_sub(1)/limit*limit);
    Ok(json!({"rows":rows.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),"total":total,"offset":offset,"limit":limit,"revision":w.revision}))
}

fn replacement_state(w: &Workspace, path: &Path, old_id: &str, new_id: &str) -> Result<(Entry, midi_file::MidiFile),String> {
    let old = w.retired_entries.get(&record_key(old_id,path)).filter(|e|e.content_id.as_deref()==Some(old_id)).cloned().ok_or("旧曲目身份已改变，请刷新文件位置")?;
    let current=w.entries.get(&key(path)).ok_or("当前文件尚未索引，请先检查文件位置")?;
    if current.content_id.as_deref()!=Some(new_id) { return Err("当前曲目身份已改变，请刷新文件位置".into()); }
    let stat=std::fs::metadata(path).map_err(|e|format!("当前文件无法读取：{e}"))?;
    if !stat.is_file() || stat.len()>32_000_000 { return Err("当前 MIDI 需要是 32 MB 以内的文件".into()); }
    let file=midi_file::MidiFile::new(path)?;
    if file.content_id!=new_id { return Err("MIDI 文件在索引后发生变化，请先重新检查文件位置".into()); }
    Ok((old,file))
}

fn notes(file:&midi_file::MidiFile)->BTreeMap<(usize,u8,u8,u8,u64,u64),usize>{
    let mut notes=BTreeMap::new();
    for track in file.tracks.iter(){for n in track.notes.iter(){
        *notes.entry((track.track_id,n.note,n.velocity,n.channel,n.start.as_micros().min(u64::MAX as u128) as u64,n.end.as_micros().min(u64::MAX as u128) as u64)).or_default()+=1;
    }}
    notes
}

pub fn preview_replacement(data:&Path,path:&Path,old_id:&str,new_id:&str)->Result<Value,String>{
    let lock=lock_for(data)?;
    let _guard=lock.lock().map_err(|_|"文件处理信息正忙")?;
    let w=Workspace::load(data)?;
    let (old,new_file)=replacement_state(&w,path,old_id,new_id)?;
    let candidates=replacement_candidates(&w,old_id,path);
    let old_file=candidates.iter().find_map(|p|midi_file::MidiFile::new(p).ok());
    let exact=old_file.as_ref().map(|file|{
        let a=notes(file);let b=notes(&new_file);
        let common=a.iter().map(|(k,count)|count.min(b.get(k).unwrap_or(&0))).sum::<usize>();
        json!({"matched":common,"oldOnly":a.values().sum::<usize>()-common,"newOnly":b.values().sum::<usize>()-common})
    });
    let token=blake3::hash(format!("{}|{}|{}|{}",w.revision,key(path),old_id,new_id).as_bytes()).to_hex().to_string();
    Ok(json!({"path":path,"oldContentId":old_id,"newContentId":new_id,"oldNotes":old.notes,"newNotes":new_file.tracks.iter().map(|t|t.notes.len()).sum::<usize>(),"oldDuration":old.duration,"newDuration":new_file.duration.as_secs_f64(),"oldTracks":old.tracks,"newTracks":new_file.tracks.iter().filter(|t|!t.notes.is_empty()).count(),"exact":exact,"oldSource":old_file.as_ref().and_then(|f|f.source_path.clone()),"candidates":candidates,"token":token,"revision":w.revision,"review":w.replacement_reviews.get(&record_key(old_id,path))}))
}

pub fn decide_replacement(data:&Path,path:&Path,old_id:&str,new_id:&str,token:&str,action:&str,target:Option<PathBuf>)->Result<Value,String>{
    if !matches!(action,"keep"|"new"|"relink"){return Err("请选择有效的文件处理方式".into());}
    let lock=lock_for(data)?;
    let _guard=lock.lock().map_err(|_|"文件处理正忙")?;
    let mut w=Workspace::load(data)?;
    let expected=blake3::hash(format!("{}|{}|{}|{}",w.revision,key(path),old_id,new_id).as_bytes()).to_hex().to_string();
    if expected!=token{return Err("文件位置在预览后发生变化，请重新预览".into());}
    replacement_state(&w,path,old_id,new_id)?;
    let target=if action=="relink"{
        let to=target.ok_or("请选择保存旧曲目内容的位置")?;
        if !replacement_candidates(&w,old_id,path).iter().any(|p|key(p)==key(&to)){return Err("所选位置不再包含旧曲目内容，请重新预览".into());}
        relink_one(&mut w,&RelinkItem{content_id:old_id.into(),from:path.into(),to:to.clone()})?;
        Some(to)
    }else{None};
    let new_key=record_key(new_id,path);
    if action=="keep" {w.ignored_paths.insert(new_key);}else{w.ignored_paths.remove(&new_key);}
    w.replacement_reviews.insert(record_key(old_id,path),ReplacementReview{old_content_id:old_id.into(),new_content_id:new_id.into(),action:action.into(),target});
    w.save(data)?;
    Ok(json!({"ok":true,"revision":w.revision,"action":action}))
}
pub fn page_size() -> usize {
    50
}
pub fn record_key(id: &str, path: &Path) -> String {
    format!("{id}|{}", key(path))
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    path: PathBuf,
    present: bool,
    available: bool,
    verified: bool,
    reason: String,
    resolved: bool,
    ignored: bool,
    relinked_to: Option<PathBuf>,
    primary: bool,
    rating: u8,
    has_score: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Group {
    content_id: String,
    title: String,
    primary_path: Option<PathBuf>,
    preferred_path: Option<PathBuf>,
    paths: Vec<Location>,
    candidates: Vec<PathBuf>,
    missing: usize,
    unverified: usize,
    available: usize,
    resolved: usize,
}
pub fn source_entry(w: &Workspace, id: &str, path: &Path) -> Option<Entry> {
    w.entries
        .get(&key(path))
        .filter(|e| e.content_id.as_deref() == Some(id))
        .cloned()
        .or_else(|| w.retired_entries.get(&record_key(id, path)).cloned())
}
pub fn link_target<'a>(w: &'a Workspace, id: &str, path: &Path) -> Option<&'a Path> {
    w.path_relinks
        .get(&record_key(id, path))
        .filter(|r| {
            r.to.is_file()
                && w.entries
                    .get(&key(&r.to))
                    .is_some_and(|e| e.content_id.as_deref() == Some(id))
        })
        .map(|r| r.to.as_path())
}
pub fn query(
    data: &Path,
    known: &Value,
    query: &str,
    kind: &str,
    include_resolved: bool,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    let lock = lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件处理信息正忙")?;
    let mut w = Workspace::load(data)?;
    let mut seeded = false;
    let mut titles = BTreeMap::new();
    let mut history_paths = Vec::new();
    for h in known["songs"].as_array().into_iter().flatten() {
        if h["generated"].as_bool() == Some(true) {
            continue;
        }
        let (Some(id), Some(path)) = (h["contentId"].as_str(), h["storedPath"].as_str()) else {
            continue;
        };
        titles.insert(
            id.to_string(),
            h["title"].as_str().unwrap_or("曲目").to_string(),
        );
        let path = PathBuf::from(path);
        history_paths.push((id.to_string(), path.clone()));
        if !w.entries.contains_key(&key(&path)) {
            let mut e = Entry {
                path: path.clone(),
                content_id: Some(id.into()),
                available: path.is_file(),
                ..Default::default()
            };
            e.metadata.title = h["title"].as_str().map(String::from);
            w.entries.insert(key(&path), e);
            seeded = true;
        }
    }
    if seeded {
        w.save(data)?;
    }
    let mut content_paths: BTreeMap<String, BTreeMap<String, PathBuf>> = BTreeMap::new();
    for e in w.entries.values().chain(w.retired_entries.values()) {
        if let Some(id) = e.content_id.as_ref() {
            content_paths
                .entry(id.clone())
                .or_default()
                .insert(key(&e.path), e.path.clone());
        }
    }
    for (id, path) in history_paths {
        content_paths
            .entry(id)
            .or_default()
            .insert(key(&path), path);
    }
    for r in w.path_relinks.values() {
        let paths = content_paths.entry(r.content_id.clone()).or_default();
        paths.insert(key(&r.from), r.from.clone());
        paths.insert(key(&r.to), r.to.clone());
    }
    let mut groups = Vec::new();
    let replacement_count=w.retired_entries.iter().filter(|(record,old)|{
        let Some(old_id)=old.content_id.as_ref() else {return false};
        w.entries.get(&key(&old.path)).and_then(|e|e.content_id.as_ref()).is_some_and(|new_id|new_id!=old_id && !w.replacement_reviews.get(*record).is_some_and(|r|&r.new_content_id==new_id))
    }).count();
    let mut missing_files = 0;
    let mut duplicate_groups = 0;
    let mut unverified_files = 0;
    let mut resolved_files = 0;
    for (id, paths) in content_paths {
        let primary = library::available_path(&w, &id, None);
        let mut locations = Vec::new();
        let mut candidates = Vec::new();
        for path in paths.values() {
            let actual = w.entries.get(&key(path));
            let e = source_entry(&w, &id, path);
            let stat = std::fs::metadata(path).ok();
            let present = stat.as_ref().is_some_and(|s| s.is_file());
            let modified = stat
                .as_ref()
                .and_then(|s| s.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos().min(u64::MAX as u128) as u64);
            let verified = actual.is_some_and(|e| {
                e.analysis_version == 2
                    && e.content_id.is_some()
                    && (!present
                        || (e.modified == modified
                            && stat.as_ref().is_some_and(|s| s.len() == e.size)))
            });
            let available =
                present && verified && actual.is_some_and(|e| e.content_id.as_deref() == Some(&id));
            if available {
                candidates.push(path.clone());
            }
            let target = link_target(&w, &id, path).map(Path::to_owned);
            let resolved = target.is_some();
            let ignored = w.ignored_paths.contains(&record_key(&id, path));
            let reason = if !present {
                "文件缺失"
            } else if !verified {
                "等待内容核对"
            } else if !available {
                "该位置内容已改变"
            } else if actual.is_some_and(|e| e.error.is_some()) {
                "附加信息需要处理"
            } else {
                "可用"
            }
            .to_string();
            locations.push(Location {
                path: path.clone(),
                present,
                available,
                verified,
                reason,
                resolved,
                ignored,
                relinked_to: target,
                primary: primary.as_ref().is_some_and(|p| key(p) == key(path)),
                rating: e.as_ref().map_or(0, |e| e.rating),
                has_score: e.as_ref().is_some_and(|e| e.has_score),
            });
        }
        let missing = locations
            .iter()
            .filter(|p| (!p.present || (p.verified && !p.available)) && !p.resolved && !p.ignored)
            .count();
        let unverified = locations
            .iter()
            .filter(|p| p.present && !p.verified && !p.resolved && !p.ignored)
            .count();
        let available = candidates.len();
        let resolved = locations.iter().filter(|p| p.resolved || p.ignored).count();
        missing_files += missing;
        unverified_files += unverified;
        resolved_files += resolved;
        if available > 1 {
            duplicate_groups += 1;
        }
        let title = primary
            .as_ref()
            .and_then(|p| source_entry(&w, &id, p))
            .and_then(|e| e.metadata.title)
            .or_else(|| {
                paths
                    .values()
                    .find_map(|p| source_entry(&w, &id, p).and_then(|e| e.metadata.title))
            })
            .or_else(|| titles.get(&id).cloned())
            .unwrap_or_else(|| {
                paths
                    .values()
                    .next()
                    .and_then(|p| p.file_stem())
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into()
            });
        let chosen = match kind {
            "duplicates" => available > 1,
            "unverified" => unverified > 0,
            "resolved" => resolved > 0,
            _ => missing > 0 || (include_resolved && resolved > 0),
        };
        if !chosen {
            continue;
        }
        let text = format!(
            "{title} {}",
            paths
                .values()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
        .to_lowercase();
        if !query
            .to_lowercase()
            .split_whitespace()
            .all(|t| text.contains(t))
        {
            continue;
        }
        let preferred_path = w.preferred_paths.get(&id).cloned();
        groups.push(Group {
            content_id: id,
            title,
            primary_path: primary,
            preferred_path,
            paths: locations,
            candidates,
            missing,
            unverified,
            available,
            resolved,
        });
    }
    groups.sort_by(|a, b| a.title.cmp(&b.title).then(a.content_id.cmp(&b.content_id)));
    let total = groups.len();
    let limit = limit.clamp(1, 100);
    let offset = offset.min(total.saturating_sub(1) / limit * limit);
    Ok(
        json!({"groups":groups.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),"total":total,"missingFiles":missing_files,"duplicateGroups":duplicate_groups,"unverifiedFiles":unverified_files,"resolvedFiles":resolved_files,"replacementCount":replacement_count,"offset":offset,"limit":limit,"revision":w.revision}),
    )
}
fn fill_metadata(
    target: &mut neothesia_core::library::SongMetadata,
    source: &neothesia_core::library::SongMetadata,
) {
    for (a, b) in [
        (&mut target.title, &source.title),
        (&mut target.composer, &source.composer),
        (&mut target.artist, &source.artist),
        (&mut target.collection, &source.collection),
        (&mut target.difficulty, &source.difficulty),
        (&mut target.notes, &source.notes),
    ] {
        if a.is_none() {
            *a = b.clone();
        }
    }
    for tag in &source.tags {
        if !target.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            target.tags.push(tag.clone());
        }
    }
}
fn relink_one(w: &mut Workspace, item: &RelinkItem) -> Result<Vec<String>, String> {
    if key(&item.from) == key(&item.to) {
        return Err("原位置与新位置相同".into());
    }
    let source = source_entry(w, &item.content_id, &item.from)
        .ok_or("原曲目身份不在文件记录中，请先核对或导入原记录")?;
    let stat = std::fs::metadata(&item.to).map_err(|e| format!("新位置无法读取：{e}"))?;
    if !stat.is_file() || stat.len() > 32_000_000 {
        return Err("新位置需要是 32 MB 以内的 MIDI 文件".into());
    }
    let parsed = midi_file::MidiFile::new(&item.to)?;
    if parsed.content_id != item.content_id {
        if w.entries.contains_key(&key(&item.to)) {
            w.inspect(item.to.clone(), true)?;
        }
        return Err("新位置与原曲目的 MIDI 内容不同，未关联".into());
    }
    let mut warnings = Vec::new();
    let mut annotation_ok = true;
    if let Err(e) = library::recover_annotations(&item.from, &item.to, &item.content_id) {
        warnings.push(format!("附加信息未合并：{e}"));
        annotation_ok = false;
    }
    let mut target = w.inspect(item.to.clone(), true)?;
    if target.content_id.as_deref() != Some(item.content_id.as_str()) {
        return Err("新位置内容在处理期间改变，未关联".into());
    }
    for group in &source.groups {
        if w.groups.iter().any(|g| &g.id == group) && !target.groups.contains(group) {
            target.groups.push(group.clone());
        }
    }
    target.rating = target.rating.max(source.rating);
    fill_metadata(&mut target.metadata, &source.metadata);
    if annotation_ok {
        if let Err(e) = neothesia_core::library::save_song_metadata(
            &item.to,
            &item.content_id,
            target.metadata.clone(),
        ) {
            warnings.push(format!("元数据未写入：{e}"));
        }
    }
    w.entries.insert(key(&item.to), target.clone());
    for alias in w
        .entries
        .values_mut()
        .filter(|e| e.content_id.as_deref() == Some(item.content_id.as_str()))
    {
        alias.groups = target.groups.clone();
        alias.rating = target.rating;
    }
    w.ignored_paths.remove(&record_key(&item.content_id,&item.from));
    w.ignored_paths.remove(&record_key(&item.content_id,&item.to));
    w.path_relinks
        .remove(&record_key(&item.content_id, &item.to));
    w.path_relinks.insert(
        record_key(&item.content_id, &item.from),
        RelinkRecord {
            content_id: item.content_id.clone(),
            from: item.from.clone(),
            to: item.to.clone(),
        },
    );
    w.preferred_paths
        .insert(item.content_id.clone(), item.to.clone());
    Ok(warnings)
}
pub fn relink(data: &Path, items: Vec<RelinkItem>) -> Result<Value, String> {
    if items.is_empty() || items.len() > 100 {
        return Err("一次选择 1～100 个文件位置".into());
    }
    let lock = lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件处理正忙")?;
    let mut w = Workspace::load(data)?;
    let mut results = Vec::new();
    let mut seen = BTreeSet::new();
    for item in items {
        if !seen.insert(record_key(&item.content_id, &item.from)) {
            continue;
        }
        let result = relink_one(&mut w, &item);
        results.push(match result{Ok(warnings)=>json!({"contentId":item.content_id,"from":item.from,"to":item.to,"ok":true,"warnings":warnings}),Err(error)=>json!({"contentId":item.content_id,"from":item.from,"to":item.to,"ok":false,"error":error})});
    }
    w.save(data)?;
    Ok(json!({"results":results,"revision":w.revision}))
}
pub fn set_primary(data: &Path, id: &str, path: Option<PathBuf>) -> Result<Value, String> {
    let lock = lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件处理正忙")?;
    let mut w = Workspace::load(data)?;
    if let Some(path) = path {
        let e = w.entries.get(&key(&path)).ok_or("位置尚未索引")?;
        if e.content_id.as_deref() != Some(id) {
            return Err("该位置不属于这首曲目".into());
        }
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 32_000_000 {
            return Err("MIDI 超过 32 MB".into());
        }
        let file = midi_file::MidiFile::new(&path)?;
        if file.content_id != id {
            return Err("该位置的实际内容已改变，未设定".into());
        }
        w.preferred_paths.insert(id.into(), path);
    } else {
        w.preferred_paths.remove(id);
    }
    w.save(data)?;
    Ok(json!({"ok":true}))
}
pub fn ignore_path(data: &Path, id: &str, path: &Path) -> Result<Value, String> {
    let lock = lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件处理正忙")?;
    let mut w = Workspace::load(data)?;
    if source_entry(&w, id, path).is_none() {
        return Err("旧位置没有已知的曲目身份，请先核对内容".into());
    }
    if path.is_file()
        && std::fs::metadata(path).is_ok_and(|s| s.len() <= 32_000_000)
        && midi_file::MidiFile::new(path).is_ok_and(|f| f.content_id == id)
    {
        return Err("该位置仍有可用曲目，请使用合并显示".into());
    }
    w.ignored_paths.insert(record_key(id, path));
    w.save(data)?;
    Ok(json!({"ok":true}))
}
pub fn restore_path(data: &Path, id: &str, path: &Path) -> Result<Value, String> {
    let lock = lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件处理正忙")?;
    let mut w = Workspace::load(data)?;
    w.path_relinks.remove(&record_key(id, path));
    w.ignored_paths.remove(&record_key(id, path));
    w.save(data)?;
    Ok(json!({"ok":true}))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, PathBuf, PathBuf, String) {
        let root = std::env::temp_dir().join(format!(
            "neothesia-repair-{}-{:?}-{}",
            std::process::id(),
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let old = root.join("old.mid");
        let new = root.join("new.mid");
        std::fs::write(&old, include_bytes!("../assets/five-finger.mid")).unwrap();
        std::fs::copy(&old, &new).unwrap();
        let mut w = Workspace::default();
        let e = w.inspect(old.clone(), false).unwrap();
        let id = e.content_id.unwrap();
        let group = w.group(None, "课后练习".into(), None).unwrap();
        w.assign(vec![old.clone()], Some(group), Some(4)).unwrap();
        w.inspect(new.clone(), false).unwrap();
        w.save(&root).unwrap();
        (root, old, new, id)
    }
    #[test]
    fn relinks_keep_identity_grouping_primary_annotations_and_restore_display() {
        let (data, old, new, id) = fixture();
        let mut side = neothesia_core::library::SongSidecar::default();
        side.fingerings.push(neothesia_core::library::FingerHint {
            track_id: 1,
            note_index: 0,
            finger: 4,
        });
        side.metadata.title = Some("教学曲".into());
        neothesia_core::library::save_song_sidecar(&old, &id, side).unwrap();
        std::fs::remove_file(&old).unwrap();
        let r = relink(
            &data,
            vec![RelinkItem {
                content_id: id.clone(),
                from: old.clone(),
                to: new.clone(),
            }],
        )
        .unwrap();
        assert_eq!(r["results"][0]["ok"], true);
        let w = Workspace::load(&data).unwrap();
        assert_eq!(w.entries[&key(&new)].rating, 4);
        assert_eq!(w.entries[&key(&new)].groups.len(), 1);
        assert_eq!(
            library::available_path(&w, &id, Some(&old)),
            Some(new.clone())
        );
        assert_eq!(
            neothesia_core::library::load_song_sidecar(&new, &id)
                .unwrap()
                .fingerings[0]
                .finger,
            4
        );
        let q = query(&data, &json!({"songs":[]}), "", "missing", false, 0, 50).unwrap();
        assert_eq!(q["missingFiles"], 0);
        assert_eq!(q["resolvedFiles"], 1);
        restore_path(&data, &id, &old).unwrap();
        assert_eq!(
            query(&data, &json!({"songs":[]}), "", "missing", false, 0, 50).unwrap()["missingFiles"],
            1
        );
    }
    #[test]
    fn replacement_archive_keeps_old_identity_and_invalid_middle_never_inherits_groups() {
        let (data, old, new, id) = fixture();
        std::fs::write(&old, b"unfinished").unwrap();
        let mut w = Workspace::load(&data).unwrap();
        w.inspect(old.clone(), true).unwrap();
        std::fs::write(&old, include_bytes!("../assets/basic-chords.mid")).unwrap();
        let changed = w.inspect(old.clone(), true).unwrap();
        assert_ne!(changed.content_id.as_ref(), Some(&id));
        assert!(changed.groups.is_empty());
        assert_eq!(changed.rating, 0);
        w.save(&data).unwrap();
        let result = relink(
            &data,
            vec![RelinkItem {
                content_id: id.clone(),
                from: old.clone(),
                to: new.clone(),
            }],
        )
        .unwrap();
        assert_eq!(result["results"][0]["ok"], true);
        let w = Workspace::load(&data).unwrap();
        assert_eq!(w.entries[&key(&old)].content_id, changed.content_id);
        assert_eq!(w.entries[&key(&new)].rating, 4);
    }
    #[test]
    fn replacement_review_keeps_identities_separate_and_rejects_stale_files() {
        let (data, path, duplicate, old_id) = fixture();
        let mut side = neothesia_core::library::SongSidecar::default();
        side.fingerings.push(neothesia_core::library::FingerHint { track_id:1,note_index:0,finger:3 });
        neothesia_core::library::save_song_sidecar(&path,&old_id,side).unwrap();
        std::fs::write(&path,include_bytes!("../assets/basic-chords.mid")).unwrap();
        let mut w=Workspace::load(&data).unwrap();
        let current=w.inspect(path.clone(),true).unwrap();
        let new_id=current.content_id.unwrap();
        assert_ne!(old_id,new_id);
        assert!(current.groups.is_empty());
        w.save(&data).unwrap();
        let listed=replacements(&data,"",false,0,25).unwrap();
        assert_eq!(listed["total"],1);
        let preview=preview_replacement(&data,&path,&old_id,&new_id).unwrap();
        assert_eq!(preview["exact"]["matched"].as_u64().is_some(),true);
        assert!(preview["exact"]["oldOnly"].as_u64().unwrap()>0);
        let stale=preview["token"].as_str().unwrap().to_string();
        std::fs::write(&path,include_bytes!("../assets/five-finger.mid")).unwrap();
        assert!(decide_replacement(&data,&path,&old_id,&new_id,&stale,"new",None).is_err());
        std::fs::write(&path,include_bytes!("../assets/basic-chords.mid")).unwrap();
        let token=preview_replacement(&data,&path,&old_id,&new_id).unwrap()["token"].as_str().unwrap().to_string();
        decide_replacement(&data,&path,&old_id,&new_id,&token,"keep",None).unwrap();
        assert!(!library::enriched(&[],&data).iter().any(|r|r.path==path));
        let reopened=Workspace::load(&data).unwrap();
        assert_eq!(reopened.replacement_reviews[&record_key(&old_id,&path)].action,"keep");
        assert_eq!(replacements(&data,"",false,0,25).unwrap()["total"],0);
        assert_eq!(replacements(&data,"",true,0,25).unwrap()["total"],1);
        let token=preview_replacement(&data,&path,&old_id,&new_id).unwrap()["token"].as_str().unwrap().to_string();
        decide_replacement(&data,&path,&old_id,&new_id,&token,"new",None).unwrap();
        assert!(library::enriched(&[],&data).iter().any(|r|r.path==path));
        let token=preview_replacement(&data,&path,&old_id,&new_id).unwrap()["token"].as_str().unwrap().to_string();
        decide_replacement(&data,&path,&old_id,&new_id,&token,"relink",Some(duplicate.clone())).unwrap();
        let reopened=Workspace::load(&data).unwrap();
        assert_eq!(reopened.replacement_reviews[&record_key(&old_id,&path)].action,"relink");
        assert_eq!(reopened.entries[&key(&path)].content_id.as_deref(),Some(new_id.as_str()));
        assert_eq!(library::available_path(&reopened,&old_id,None),Some(duplicate.clone()));
        assert_eq!(neothesia_core::library::load_song_sidecar(&duplicate,&old_id).unwrap().fingerings[0].finger,3);
        assert!(neothesia_core::library::load_song_sidecar(&path,&new_id).is_err());
    }
    #[test]
    fn batches_report_mismatches_and_primary_checks_actual_content() {
        let (data, old, new, id) = fixture();
        let bad = data.join("wrong.mid");
        std::fs::write(&bad, include_bytes!("../assets/basic-chords.mid")).unwrap();
        let result = relink(
            &data,
            vec![
                RelinkItem {
                    content_id: id.clone(),
                    from: old.clone(),
                    to: bad,
                },
                RelinkItem {
                    content_id: id.clone(),
                    from: new.clone(),
                    to: old.clone(),
                },
            ],
        )
        .unwrap();
        assert_eq!(result["results"][0]["ok"], false);
        assert_eq!(result["results"][1]["ok"], true);
        std::fs::write(&old, include_bytes!("../assets/basic-chords.mid")).unwrap();
        assert!(set_primary(&data, &id, Some(old)).is_err());
    }
    #[test]
    fn reversing_a_display_merge_keeps_one_visible_location_and_all_files() {
        let (data, old, new, id) = fixture();
        relink(
            &data,
            vec![RelinkItem {
                content_id: id.clone(),
                from: old.clone(),
                to: new.clone(),
            }],
        )
        .unwrap();
        relink(
            &data,
            vec![RelinkItem {
                content_id: id.clone(),
                from: new.clone(),
                to: old.clone(),
            }],
        )
        .unwrap();
        let visible = library::enriched(&[], &data);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].path, old);
        assert!(new.is_file());
        assert!(old.is_file());
        let w = Workspace::load(&data).unwrap();
        assert_eq!(
            library::verified_file(&data, &id, [new.clone()])
                .unwrap()
                .source_path,
            Some(old)
        );
        assert_eq!(w.path_relinks.len(), 1);
    }
    #[test]
    fn ignored_old_identity_is_reversible_and_never_hides_a_replacement() {
        let (data, old, _new, id) = fixture();
        assert!(ignore_path(&data, &id, &old).is_err());
        std::fs::write(&old, include_bytes!("../assets/basic-chords.mid")).unwrap();
        let mut w = Workspace::load(&data).unwrap();
        w.inspect(old.clone(), true).unwrap();
        w.save(&data).unwrap();
        ignore_path(&data, &id, &old).unwrap();
        let q = query(&data, &json!({}), "", "missing", false, 0, 50).unwrap();
        assert_eq!(q["missingFiles"], 0);
        assert_eq!(q["resolvedFiles"], 1);
        assert!(library::enriched(&[], &data).iter().any(|r| r.path == old));
        assert!(old.is_file());
        restore_path(&data, &id, &old).unwrap();
        assert_eq!(
            query(&data, &json!({}), "", "missing", false, 0, 50).unwrap()["missingFiles"],
            1
        );
    }
}
