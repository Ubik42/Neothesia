use midi_file::MidiFile;
use neothesia_core::library::{SongMetadata, load_song_sidecar, save_song_metadata};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Workspace {
    pub groups: Vec<Group>,
    pub entries: BTreeMap<String, Entry>,
    pub revision: u64,
    pub learning: BTreeMap<String,crate::song_learning::Learning>,
    pub preferred_paths:BTreeMap<String,PathBuf>,
    pub ignored_paths:std::collections::BTreeSet<String>,
    pub path_relinks:BTreeMap<String,crate::library_repairs::RelinkRecord>,
    pub retired_entries:BTreeMap<String,Entry>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicates_keep_groups_and_replaced_content_does_not_inherit_them() {
        let root = std::env::temp_dir().join(format!(
            "neothesia-library-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("one.mid");
        let second = root.join("renamed.mid");
        std::fs::write(&first, include_bytes!("../assets/five-finger.mid")).unwrap();
        std::fs::copy(&first, &second).unwrap();
        let mut w = Workspace::default();
        let top = w.group(None, "曲集".into(), None).unwrap();
        let child = w.group(None, "练习".into(), Some(top.clone())).unwrap();
        assert!(
            w.group(Some(top.clone()), "曲集".into(), Some(child.clone()))
                .is_err()
        );
        w.inspect(first.clone(), false).unwrap();
        w.assign(vec![first.clone()], Some(child.clone()), Some(4))
            .unwrap();
        assert_eq!(
            w.inspect(second.clone(), false).unwrap().groups,
            vec![child.clone()]
        );
        std::fs::write(&second, include_bytes!("../assets/basic-chords.mid")).unwrap();
        let e = w.inspect(second, false).unwrap();
        assert!(e.groups.is_empty());
        assert_eq!(e.rating, 0);
        w.save(&root).unwrap();
        let reopened = Workspace::load(&root).unwrap();
        assert_eq!(reopened.entries[&key(&first)].groups, vec![child]);
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Entry {
    pub path: PathBuf,
    pub content_id: Option<String>,
    pub metadata: SongMetadata,
    pub groups: Vec<String>,
    pub rating: u8,
    pub has_score: bool,
    pub score_name: Option<String>,
    pub notes: usize,
    pub duration: f64,
    pub tracks: usize,
    pub measures: usize,
    pub meter: String,
    pub available: bool,
    pub error: Option<String>,
    pub size: u64,
    pub modified: u64,
    pub annotation_modified: u64,
    pub analysis_version: u16,
}
pub fn key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}
pub fn lock_for(data: &Path) -> Result<std::sync::Arc<std::sync::Mutex<()>>, String> {
    static LOCKS: std::sync::OnceLock<
        std::sync::Mutex<BTreeMap<String, std::sync::Arc<std::sync::Mutex<()>>>>,
    > = std::sync::OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "曲库锁不可用")?;
    Ok(locks.entry(key(data)).or_default().clone())
}
impl Workspace {
    pub fn load(data: &Path) -> Result<Self, String> {
        match std::fs::read(data.join("library-workspace.json")) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("曲库管理信息无法读取：{e}"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn save(&mut self, data: &Path) -> Result<(), String> {
        self.revision += 1;
        std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
        let path = data.join("library-workspace.json");
        let temp = path.with_extension("json.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(temp, path).map_err(|e| e.to_string())
    }
    pub fn inspect(&mut self, path: PathBuf, force: bool) -> Result<Entry, String> {
        let k = key(&path);
        let previous = self.entries.get(&k).cloned().unwrap_or_default();
        let previous_identity=previous.clone();
        let mut entry = Entry {
            path: path.clone(),
            ..previous
        };
        let stat = match std::fs::metadata(&path) {
            Ok(s) => s,
            Err(e) => {
                entry.available = false;
                entry.error = Some(format!("文件无法读取：{e}"));
                self.entries.insert(k, entry.clone());
                return Ok(entry);
            }
        };
        let size = stat.len();
        let modified = stat
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos().min(u64::MAX as u128) as u64);
        let annotation_modified =
            std::fs::metadata(neothesia_core::library::metadata_sidecar_path(&path))
                .ok()
                .and_then(|s| s.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos().min(u64::MAX as u128) as u64);
        if !force
            && entry.analysis_version == 2
            && entry.annotation_modified == annotation_modified
            && entry.content_id.is_some()
            && entry.size == size
            && entry.modified == modified
            && entry.error.is_none()
        {
            entry.available = true;
            self.entries.insert(k, entry.clone());
            return Ok(entry);
        }
        entry.available = true;
        entry.size = size;
        entry.modified = modified;
        entry.annotation_modified = annotation_modified;
        entry.analysis_version = 2;
        entry.error = None;
        if size > 32_000_000 {
            entry.error = Some("MIDI 超过 32 MB，请使用更小的文件".into());
            self.entries.insert(k, entry.clone());
            return Ok(entry);
        }
        match MidiFile::new(&path) {
            Ok(file) => {
                if entry.content_id.as_ref().is_some_and(|id|id!=&file.content_id)
                    || (entry.content_id.is_none() && self.retired_entries.values().any(|e|key(&e.path)==k)) {
                    if let Some(id)=previous_identity.content_id.as_ref(){self.retired_entries.insert(crate::library_repairs::record_key(id,&path),previous_identity.clone());}
                    entry.groups.clear();
                    entry.rating = 0;
                    entry.metadata = SongMetadata::default();
                }
                if let Some(alias) = self
                    .entries
                    .values()
                    .find(|v| v.content_id.as_ref() == Some(&file.content_id) && key(&v.path) != k)
                {
                    for group in &alias.groups {
                        if !entry.groups.contains(group) {
                            entry.groups.push(group.clone());
                        }
                    }
                    entry.rating = alias.rating;
                    entry.metadata = alias.metadata.clone();
                }
                entry.content_id = Some(file.content_id.clone());
                entry.notes = file.tracks.iter().map(|t| t.notes.len()).sum();
                entry.tracks = file.tracks.iter().filter(|t| !t.notes.is_empty()).count();
                entry.duration = file
                    .tracks
                    .iter()
                    .flat_map(|t| t.notes.iter())
                    .map(|n| n.end.as_secs_f64())
                    .fold(0., f64::max);
                entry.measures = file
                    .musical_time
                    .measures
                    .iter()
                    .filter(|m| {
                        file.tempo_track
                            .pulses_to_duration(m.start_tick)
                            .as_secs_f64()
                            < entry.duration
                    })
                    .count();
                let meters: Vec<_> = file
                    .musical_time
                    .measures
                    .iter()
                    .map(|m| {
                        format!(
                            "{}/{}{}",
                            m.numerator,
                            m.denominator,
                            if m.explicit_meter { "" } else { " 默认" }
                        )
                    })
                    .fold(Vec::new(), |mut acc, s| {
                        if !acc.contains(&s) {
                            acc.push(s)
                        }
                        acc
                    });
                entry.meter = meters.join(" → ");
                match load_song_sidecar(&path, &file.content_id) {
                    Ok(s) => {
                        if let Some(meter) = s.meter {
                            let end =
                                file.tempo_track.seconds_to_pulses(entry.duration).round() as u64;
                            match midi_file::musical_time::MusicalTime::corrected(
                                file.musical_time.ppq,
                                end,
                                meter.numerator,
                                meter.denominator,
                                meter.pickup_ticks,
                            ) {
                                Ok(grid) => {
                                    entry.measures = grid.measures.len();
                                    entry.meter = format!(
                                        "{}/{} 手动{}",
                                        meter.numerator,
                                        meter.denominator,
                                        if meter.pickup_ticks > 0 {
                                            " · 弱起"
                                        } else {
                                            ""
                                        }
                                    );
                                }
                                Err(e) => entry.error = Some(format!("小节网格：{e}")),
                            }
                        }
                        entry.metadata = s.metadata;
                        entry.has_score = s.score.is_some();
                        entry.score_name = s.score.map(|s| s.path.to_string_lossy().into());
                    }
                    Err(neothesia_core::library::MetadataError::Read(e))
                        if e.kind() == std::io::ErrorKind::NotFound =>
                    {
                        entry.has_score = false;
                        entry.score_name = None;
                    }
                    Err(e) => {
                        entry.error = Some(format!("附加信息：{e}"));
                    }
                }
            }
            Err(e) => {
                if let Some(id)=previous_identity.content_id.as_ref(){self.retired_entries.insert(crate::library_repairs::record_key(id,&path),previous_identity.clone());}
                entry.content_id = None;
                entry.error = Some(e.to_string());
            }
        }
        self.entries.insert(k, entry.clone());
        Ok(entry)
    }
    pub fn group(
        &mut self,
        id: Option<String>,
        name: String,
        parent: Option<String>,
    ) -> Result<String, String> {
        let name = name.trim().to_string();
        if name.is_empty() || name.chars().count() > 80 {
            return Err("分组名称需要 1～80 个字".into());
        }
        if parent
            .as_ref()
            .is_some_and(|p| !self.groups.iter().any(|g| &g.id == p))
        {
            return Err("上级分组不存在".into());
        }
        let id = id.unwrap_or_else(|| {
            format!(
                "group-{}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
                self.revision
            )
        });
        let mut ancestor = parent.clone();
        for _ in 0..=self.groups.len() {
            if ancestor.as_ref() == Some(&id) {
                return Err("分组不能放进自身或下级分组".into());
            }
            ancestor = ancestor.and_then(|a| {
                self.groups
                    .iter()
                    .find(|g| g.id == a)
                    .and_then(|g| g.parent.clone())
            });
        }
        if self
            .groups
            .iter()
            .any(|g| g.id != id && g.parent == parent && g.name == name)
        {
            return Err("这个位置已有同名分组".into());
        }
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == id) {
            g.name = name;
            g.parent = parent;
        } else {
            self.groups.push(Group {
                id: id.clone(),
                name,
                parent,
            });
        }
        Ok(id)
    }
    pub fn remove_group(&mut self, id: &str) -> Result<(), String> {
        let parent = self
            .groups
            .iter()
            .find(|g| g.id == id)
            .ok_or("分组不存在")?
            .parent
            .clone();
        self.groups.retain(|g| g.id != id);
        for g in &mut self.groups {
            if g.parent.as_deref() == Some(id) {
                g.parent = parent.clone();
            }
        }
        for e in self.entries.values_mut() {
            e.groups.retain(|g| g != id);
        }
        Ok(())
    }
    pub fn assign(
        &mut self,
        paths: Vec<PathBuf>,
        group: Option<String>,
        rating: Option<u8>,
    ) -> Result<(), String> {
        if group
            .as_ref()
            .is_some_and(|id| !self.groups.iter().any(|g| &g.id == id))
        {
            return Err("分组不存在".into());
        }
        if rating.is_some_and(|r| r > 5) {
            return Err("评级应为 0～5".into());
        }
        for path in paths {
            let entry = self.entries.entry(key(&path)).or_insert_with(|| Entry {
                path: path.clone(),
                available: path.is_file(),
                ..Entry::default()
            });
            if let Some(g) = &group {
                if !entry.groups.contains(g) {
                    entry.groups.push(g.clone());
                }
            }
            if let Some(r) = rating {
                entry.rating = r;
            }
            let id = entry.content_id.clone();
            let groups = entry.groups.clone();
            let rating = entry.rating;
            if let Some(id) = id {
                for alias in self
                    .entries
                    .values_mut()
                    .filter(|e| e.content_id.as_ref() == Some(&id))
                {
                    alias.groups = groups.clone();
                    alias.rating = rating;
                }
            }
        }
        Ok(())
    }
    pub fn unassign(&mut self, paths: Vec<PathBuf>, group: &str) {
        for path in paths {
            if let Some(entry) = self.entries.get_mut(&key(&path)) {
                entry.groups.retain(|g| g != group);
                let id = entry.content_id.clone();
                if let Some(id) = id {
                    for alias in self
                        .entries
                        .values_mut()
                        .filter(|e| e.content_id.as_ref() == Some(&id))
                    {
                        alias.groups.retain(|g| g != group);
                    }
                }
            }
        }
    }
    pub fn update_metadata(&mut self, path: PathBuf, value: SongMetadata) -> Result<(), String> {
        let entry = self.inspect(path.clone(), true)?;
        let id = entry
            .content_id
            .ok_or("无法识别 MIDI 内容，不能保存元数据")?;
        save_song_metadata(&path, &id, value.clone()).map_err(|e| e.to_string())?;
        for entry in self
            .entries
            .values_mut()
            .filter(|e| e.content_id.as_ref() == Some(&id))
        {
            entry.metadata = value.clone();
        }
        Ok(())
    }
}

pub fn accepts(command: &crate::Command) -> bool {
    matches!(
        command,
        crate::Command::LibraryWorkspace
            | crate::Command::SaveSongLearning{..}
            | crate::Command::InspectSong { .. }
            | crate::Command::InspectSongs { .. }
            | crate::Command::LibraryGroup { .. }
            | crate::Command::RemoveLibraryGroup { .. }
            | crate::Command::AssignLibrary { .. }
            | crate::Command::UnassignLibrary { .. }
            | crate::Command::LibraryMetadataHistory { .. } | crate::Command::PreviewMetadataRestore { .. } | crate::Command::RestoreLibraryMetadata { .. }
            | crate::Command::PreviewLibraryMetadata { .. }
            | crate::Command::ReadLibraryMetadata { .. }
            | crate::Command::SaveLibraryMetadata { .. }
            | crate::Command::UpdateSongMetadata { .. }
    )
}
pub fn execute(command: crate::Command, data: &Path) -> Result<serde_json::Value, String> {
    use crate::Command;
    let lock = lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "曲库文件正忙")?;
    let mut workspace = Workspace::load(data)?;
    if matches!(command, Command::LibraryWorkspace) {
        for e in workspace.entries.values_mut() {
            e.available = e.path.is_file();
        }
        let mut value=serde_json::to_value(workspace).map_err(|e|e.to_string())?;
        let papers=crate::score_attachments::summaries(data);
        for entry in value["entries"].as_object_mut().unwrap().values_mut(){if let Some((count,names))=entry["contentId"].as_str().and_then(|id|papers.get(id)){entry["paperCount"]=serde_json::json!(count);entry["paperNames"]=serde_json::json!(names);}}
        let preferences=crate::preferences::Preferences::load(&data.join("web-preferences.json"))?;
        for entry in value["entries"].as_object_mut().unwrap().values_mut(){let id=entry["contentId"].as_str().unwrap_or("");let versions=preferences.score_versions.get(id);entry["notationCount"]=serde_json::json!(versions.map_or(0,|v|v.len()).max(usize::from(entry["hasScore"].as_bool()==Some(true))));}
        return Ok(value);
    }
    let mut result = match command {
        Command::SaveSongLearning{path,content_id,value,expected}=>{
            crate::song_learning::save(&mut workspace,path,content_id,value,expected)?;
            serde_json::json!({"ok":true})
        },
        Command::InspectSong { path, force } => {
            serde_json::to_value(workspace.inspect(path, force)?).map_err(|e| e.to_string())?
        }
        Command::InspectSongs { paths, force } => {
            if paths.len() > 64 {
                return Err("一次最多索引 64 首曲目".into());
            }
            let mut entries = Vec::new();
            for path in paths {
                entries.push(workspace.inspect(path, force)?);
            }
            serde_json::json!({"entries":entries})
        }
        Command::LibraryGroup { id, name, parent } => {
            serde_json::json!({"id":workspace.group(id,name,parent)?})
        }
        Command::RemoveLibraryGroup { id } => {
            workspace.remove_group(&id)?;
            serde_json::json!({"ok":true})
        }
        Command::AssignLibrary {
            paths,
            group,
            rating,
        } => {
            workspace.assign(paths, group, rating)?;
            serde_json::json!({"ok":true})
        }
        Command::UnassignLibrary { paths, group } => {
            workspace.unassign(paths, &group);
            serde_json::json!({"ok":true})
        }
        Command::LibraryMetadataHistory{path,content_id}=>crate::library_metadata::history(&mut workspace,data,path,content_id)?,
        Command::PreviewMetadataRestore{path,content_id,id}=>crate::library_metadata::restore_preview(&mut workspace,data,path,content_id,id)?,
        Command::RestoreLibraryMetadata{path,content_id,id,fields,expected}=>crate::library_metadata::restore(&mut workspace,data,path,content_id,id,fields,expected)?,
        Command::PreviewLibraryMetadata{path,content_id,rules}=>crate::library_metadata::preview(&mut workspace,path,content_id,rules)?,
        Command::ReadLibraryMetadata{path,content_id}=>crate::library_metadata::read(&mut workspace,path,content_id)?,
        Command::SaveLibraryMetadata{path,content_id,value,expected}=>crate::library_metadata::save(&mut workspace,data,path,content_id,value,expected)?,
        Command::UpdateSongMetadata { path, value } => {
            workspace.update_metadata(path, value)?;
            serde_json::json!({"ok":true})
        }
        _ => unreachable!(),
    };
    if let Err(error)=workspace.save(data){if result["status"]=="saved"{let warning=serde_json::json!(format!("资料已保存，曲库索引写入失败：{error}"));if let Some(warnings)=result["warnings"].as_array_mut(){warnings.push(warning);}else{result["warnings"]=serde_json::json!([warning]);}}else{return Err(error);}}
    return Ok(result);
}
