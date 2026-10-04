use crate::{preferences::ScoreVersion, *};
use neothesia_core::library::{self as metadata, SongSidecar};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};
fn read_score(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = vec![];
    file.take(4_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4_000_000 {
        return Err("谱面超过 4 MB，请整理或重新导入".into());
    }
    Ok(bytes)
}
fn sidecar(path: &Path, cid: &str) -> Result<SongSidecar, String> {
    match metadata::load_song_sidecar(path, cid) {
        Ok(s) => Ok(s),
        Err(metadata::MetadataError::Read(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(SongSidecar::default())
        }
        Err(e) => Err(e.to_string()),
    }
}
pub(super) fn location(data: &Path, path: &Path, cid: &str) -> Result<(PathBuf, Option<MidiFile>), String> {
    if cid.is_empty() || cid.len() > 160 {
        return Err("曲目身份无效".into());
    }
    if path.is_file() && MidiFile::new(path)?.content_id != cid {
        return Err("此位置已换成另一首曲目，请重新检查文件后管理谱面".into());
    }
    match library::verified_file(data, cid, [path.to_owned()]) {
        Ok(f) => Ok((f.source_path.clone().ok_or("曲目没有本地位置")?, Some(f))),
        Err(_) => {
            let w = library_workspace::Workspace::load(data)?;
            let prefs = Preferences::load(&data.join("web-preferences.json"))?;
            if library_repairs::source_entry(&w, cid, path).is_none()
                && !prefs.songs.contains_key(cid)
                && !prefs.score_versions.contains_key(cid)
            {
                return Err("曲目未登记，请先检查文件".into());
            }
            Ok((path.to_owned(), None))
        }
    }
}
pub(super) fn versions(
    prefs: &Preferences,
    path: &Path,
    cid: &str,
) -> Result<(Vec<ScoreVersion>, Option<PathBuf>), String> {
    let association = sidecar(path, cid)?.score;
    let active = association
        .as_ref()
        .map(|a| metadata::resolve_score_path(path, a));
    let mut rows = prefs.score_versions.get(cid).cloned().unwrap_or_default();
    if let Some(a) = association {
        let path = metadata::resolve_score_path(path, &a);
        if !rows
            .iter()
            .any(|v| v.path == path && v.content_id == a.content_id)
        {
            rows.push(ScoreVersion {
                id: format!("associated-{}", a.content_id),
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                path,
                content_id: a.content_id,
            });
        }
    }
    Ok((rows, active))
}
pub fn inspect(data: &Path, path: &Path, cid: &str) -> Result<Value, String> {
    let (path, file) = location(data, path, cid)?;
    let prefs = Preferences::load(&data.join("web-preferences.json"))?;
    let (versions, active) = versions(&prefs, &path, cid)?;
    let mut copies = std::collections::BTreeMap::new();
    for v in &versions {
        *copies
            .entry((v.content_id.clone(), v.name.clone()))
            .or_insert(0usize) += 1;
    }
    let rows:Vec<_>=versions.into_iter().map(|v|{
        let (status,size,error)=match read_score(&v.path){Ok(bytes)=>(if blake3::hash(&bytes).to_hex().as_str()==v.content_id {"ready"}else{"changed"},bytes.len(),None),Err(e)=>(if !v.path.exists() {"missing"}else{"unreadable"},0,Some(e.to_string()))};
        json!({"id":v.id,"name":v.name,"copies":copies[&(v.content_id.clone(),v.name.clone())],"path":v.path,"active":active.as_ref()==Some(&v.path),"status":status,"size":size,"error":error})
    }).collect();
    let papers = score_attachments::list_verified(data, cid)?;
    Ok(
        json!({"contentId":cid,"path":path,"songAvailable":file.is_some(),"versions":rows,"papers":papers}),
    )
}
pub fn check_import(data:&Path,path:&Path,cid:&str,bytes:&[u8],kind:&str,target:Option<&str>)->Result<Value,String>{
    if bytes.is_empty() || bytes.len()>if kind=="notation"{4_000_000}else{48_000_000} {return Err("待核对文件为空或超过谱面容量上限".into());}
    if !["notation","pdf","image"].contains(&kind){return Err("谱面核对类型无效".into());}
    let (path,file)=location(data,path,cid)?;
    if file.is_none(){return Err("演奏文件不可用，请先修复位置后核对".into());}
    let hash=blake3::hash(bytes).to_hex().to_string();
    let mut matches=Vec::new();
    if kind=="notation" {
        let prefs=Preferences::load(&data.join("web-preferences.json"))?;
        for version in versions(&prefs,&path,cid)?.0 {
            if version.content_id==hash && read_score(&version.path).is_ok_and(|actual|actual==bytes) {
                matches.push(json!({"id":version.id,"name":version.name,"kind":"notation","page":0}));
            }
        }
    }else{
        let papers=score_attachments::list_verified(data,cid)?;
        for book in papers["attachments"].as_array().ok_or("纸谱目录无效")? {
            let id=book["id"].as_str().ok_or("纸谱身份无效")?;
            if target.is_some_and(|expected|expected!=id){continue;}
            for (page,asset) in book["pages"].as_array().ok_or("谱页目录无效")?.iter().enumerate(){
                if asset["id"].as_str()==Some(&hash) && asset["status"].as_str()==Some("ready") && (kind=="pdf")== (asset["kind"].as_str()==Some("pdf")) {
                    let actual=score_attachments::read(data,cid,id,page)?.0;
                    if actual==bytes {matches.push(json!({"id":id,"name":book["name"],"kind":"paper","page":page,"assetId":hash}));}
                }
            }
        }
    }
    Ok(json!({"matches":matches}))
}

impl Player {
    pub(super) fn store_score_version_file(
        &mut self,
        score: &notation::ScoreAsset,
        cid: &str,
    ) -> Result<PathBuf, String> {
        self.store_library_score_bytes(&score.bytes,&score.name,cid)
    }
    fn store_library_score_bytes(&mut self,bytes:&[u8],name:&str,cid:&str)->Result<PathBuf,String>{
        let hash = blake3::hash(bytes).to_hex().to_string();
        let file_key = cid
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>();
        let found = self
            .preferences
            .score_versions
            .get(cid)
            .and_then(|v| {
                v.iter()
                    .find(|v| v.content_id == hash && v.name == name)
            })
            .cloned();
        let directory = self.data.join("scores");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let path = if let Some(v) = &found {
            if v.path.parent() == Some(directory.as_path())
            {
                v.path.clone()
            } else {
                directory.join(format!(
                    "{}-{}.{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| e.to_string())?
                        .as_nanos(),
                    file_key,
                    if bytes.starts_with(b"PK") {
                        "mxl"
                    } else {
                        "musicxml"
                    }
                ))
            }
        } else {
            directory.join(format!(
                "{}-{}.{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos(),
                file_key,
                if bytes.starts_with(b"PK") {
                    "mxl"
                } else {
                    "musicxml"
                }
            ))
        };
        if path.parent() == Some(directory.as_path()) {
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        }
        if let Some(v) = found {
            if v.path != path {
                self.preferences
                    .score_versions
                    .get_mut(cid)
                    .unwrap()
                    .iter_mut()
                    .find(|row| row.id == v.id)
                    .unwrap()
                    .path = path.clone();
                self.preferences.save(&self.preferences_path)?;
            }
        }
        Ok(path)
    }
    pub(super) fn add_library_notation(&mut self,path:PathBuf,cid:String,name:String,bytes:Vec<u8>,repair:Option<String>)->Result<Value,String>{
       let (path,_)=location(&self.data,&path,&cid)?;
       let name=name.trim();if name.is_empty()||name.chars().count()>120{return Err("谱面名称需为 1 至 120 字".into());}
       if bytes.len()>4_000_000{return Err("演奏乐谱超过 4 MB".into());}
       let hash=blake3::hash(&bytes).to_hex().to_string();let (mut rows,active)=versions(&self.preferences,&path,&cid)?;
       let mut old_path=None;
       let (name,existing)=if let Some(id)=repair {let v=rows.iter().find(|v|v.id==id).ok_or("待修复版本不存在")?;if v.content_id!=hash{return Err("文件内容与原版本不符；请选择原文件修复，或添加为新版本".into());}old_path=Some(v.path.clone());(v.name.clone(),Some(v.id.clone()))}else{(name.to_owned(),rows.iter().find(|v|v.name==name&&v.content_id==hash).map(|v|v.id.clone()))};
       if old_path.is_none(){old_path=existing.as_ref().and_then(|id|rows.iter().find(|v|&v.id==id).map(|v|v.path.clone()));}
       if existing.is_none()&&rows.len()>=100{return Err("这首曲目已有 100 份演奏乐谱，请先整理版本".into());}
       self.preferences.score_versions.insert(cid.clone(),rows.clone());
       let stored=self.store_library_score_bytes(&bytes,&name,&cid)?;
       // The helper can migrate an unavailable external version to an owned copy.
       rows=self.preferences.score_versions.get(&cid).cloned().unwrap_or_default();
       let id=existing.unwrap_or_else(||format!("library-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
       if let Some(v)=rows.iter_mut().find(|v|v.id==id){v.path=stored.clone();}else{rows.push(ScoreVersion{id:id.clone(),name,path:stored.clone(),content_id:hash});}
       if old_path.as_ref().is_some_and(|p|active.as_ref()==Some(p)) {metadata::save_score_association(&path,&cid,&stored).map_err(|e|e.to_string())?;}
       self.preferences.score_versions.insert(cid,rows);self.preferences.save(&self.preferences_path)?;
       Ok(json!({"id":id,"updated":true}))
    }
    pub(super) fn manage_library_score(
        &mut self,
        path: PathBuf,
        cid: String,
        kind: String,
        id: String,
        action: String,
        name: Option<String>,
    ) -> Result<Value, String> {
        let (path, file) = location(&self.data, &path, &cid)?;
        if kind == "paper" {
            if !["select", "rename", "remove"].contains(&action.as_str()) {
                return Err("谱页管理操作无效".into());
            }
            return score_attachments::update(
                &self.data,
                &cid,
                Some(&id),
                &action,
                name.as_deref(),
                None,
                0,
                0,
            );
        }
        if kind != "notation"
            || !["rename", "remove", "select", "detach", "deduplicate"].contains(&action.as_str())
        {
            return Err("乐谱管理操作无效".into());
        }
        let (rows, active) = versions(&self.preferences, &path, &cid)?;
        let v = rows
            .iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or("谱面版本不存在")?;
        if rows.len() > 100 && !["remove", "detach", "deduplicate"].contains(&action.as_str()) {
            return Err("谱面版本已超过上限，请先移除或合并重复登记".into());
        }
        let current = self.file.as_ref().is_some_and(|f| f.content_id == cid);
        match action.as_str() {
            "rename" => {
                let n = name.as_deref().unwrap_or("").trim();
                if n.is_empty() || n.chars().count() > 120 {
                    return Err("请输入 1 至 120 字的谱面名称".into());
                }
                let mut rows = rows;
                rows.iter_mut().find(|a| a.id == id).unwrap().name = n.into();
                self.preferences.score_versions.insert(cid.clone(), rows);
                self.preferences.save(&self.preferences_path)?;
            }
            "deduplicate" => {
                let bytes = read_score(&v.path)?;
                if blake3::hash(&bytes).to_hex().as_str() != v.content_id {
                    return Err("所选版本不可用，请选择完整副本或重新导入".into());
                }
                let is_active_copy = active.as_ref().is_some_and(|path| {
                    rows.iter().any(|r| {
                        &r.path == path && r.name == v.name && r.content_id == v.content_id
                    })
                });
                let remaining: Vec<_> = rows
                    .into_iter()
                    .filter(|r| r.id == v.id || r.name != v.name || r.content_id != v.content_id)
                    .collect();
                if remaining.len() > 100 {
                    return Err("合并后仍超过 100 份，请先移除其他登记".into());
                }
                if is_active_copy {
                    metadata::save_score_association(&path, &cid, &v.path)
                        .map_err(|e| e.to_string())?;
                    if current {
                        let current_path = self.annotation_path(self.file.as_ref().unwrap())?;
                        metadata::save_score_association(&current_path, &cid, &v.path)
                            .map_err(|e| e.to_string())?;
                    }
                }
                self.preferences
                    .score_versions
                    .insert(cid.clone(), remaining);
                self.preferences.save(&self.preferences_path)?;
            }
            "remove" => {
                if active.as_ref() == Some(&v.path) {
                    return Err("请先解除此谱面的关联，再移除版本".into());
                }
                self.preferences.score_versions.insert(
                    cid.clone(),
                    rows.into_iter().filter(|v| v.id != id).collect(),
                );
                self.preferences.save(&self.preferences_path)?;
            }
            "select" => {
                if self.recorder.is_recording() {
                    return Err("请结束录音后切换关联谱面".into());
                }
                let mut file = file.ok_or("MIDI 文件缺失，请先重新定位；仍可修改版本名称")?;
                if let Some(m) = sidecar(&path, &cid)?.meter {
                    file.musical_time = Self::corrected_grid(&file, m)?;
                }
                let bytes = read_score(&v.path)?;
                if blake3::hash(&bytes).to_hex().as_str() != v.content_id {
                    return Err("谱面内容已改变，请重新导入确认".into());
                }
                notation::ScoreAsset::new(bytes, v.name.clone(), &file)?;
                metadata::save_score_association(&path, &cid, &v.path)
                    .map_err(|e| e.to_string())?;
                self.preferences.score_versions.insert(cid.clone(), rows);
                self.preferences.save(&self.preferences_path)?;
                if current {
                    self.command(Command::ActivateScore {
                        id,
                        content_id: cid.clone(),
                    })?;
                }
            }
            "detach" => {
                if active.as_ref() != Some(&v.path) {
                    return Err("此谱面不是当前关联".into());
                }
                metadata::clear_score_association(&path, &cid).map_err(|e| e.to_string())?;
                self.preferences.score_versions.insert(
                    cid.clone(),
                    if rows.len() > 100 {
                        rows.into_iter().filter(|r| r.id != id).collect()
                    } else {
                        rows
                    },
                );
                self.preferences.save(&self.preferences_path)?;
                if current {
                    self.command(Command::DetachScore {
                        content_id: cid.clone(),
                    })?;
                }
            }
            _ => unreachable!(),
        }
        if path.is_file() {
            let mut w = library_workspace::Workspace::load(&self.data)?;
            let association = sidecar(&path, &cid)?.score;
            if let Some(entry) = w.entries.get_mut(&library_workspace::key(&path)) {
                if entry.content_id.as_deref() == Some(&cid) {
                    entry.has_score = association.is_some();
                    entry.score_name = association.map(|a| {
                        let path = metadata::resolve_score_path(&path, &a);
                        self.preferences
                            .score_versions
                            .get(&cid)
                            .and_then(|v| v.iter().find(|v| v.path == path))
                            .map(|v| v.name.clone())
                            .unwrap_or_else(|| {
                                path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into()
                            })
                    });
                    w.save(&self.data)?;
                }
            }
        }
        // The UI refreshes file verification on the caller thread, outside the music clock.
        Ok(json!({"contentId":cid,"path":path,"updated":true}))
    }

    pub(super) fn open_library_score(
        &mut self,
        path: PathBuf,
        cid: String,
        kind: String,
        id: String,
    ) -> Result<Value, String> {
        if self.recorder.is_recording() {
            return Err("请结束录音后打开另一份谱面".into());
        }
        let (path, file) = location(&self.data, &path, &cid)?;
        let mut file = file.ok_or("曲目文件缺失，请先重新定位 MIDI")?;
        if let Some(m) = sidecar(&path, &cid)?.meter {
            file.musical_time = Self::corrected_grid(&file, m)?;
        }
        if kind == "notation" {
            let (rows, _) = versions(&self.preferences, &path, &cid)?;
            if rows.len() > 100 {
                return Err("谱面超过 100 份，请先移除或合并重复登记".into());
            }
            let v = rows.iter().find(|v| v.id == id).ok_or("谱面版本不存在")?;
            let bytes = read_score(&v.path)?;
            if blake3::hash(&bytes).to_hex().as_str() != v.content_id {
                return Err("谱面内容已改变，请重新导入确认".into());
            }
            notation::ScoreAsset::new(bytes, v.name.clone(), &file)?;
            self.preferences.score_versions.insert(cid.clone(), rows);
            self.preferences.save(&self.preferences_path)?;
        } else if kind == "paper" {
            let papers = score_attachments::list(&self.data, &cid)?;
            let book = papers["attachments"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["id"].as_str() == Some(&id))
                .ok_or("谱页版本不存在")?;
            for i in 0..book["pages"].as_array().unwrap().len() {
                score_attachments::read(&self.data, &cid, &id, i)?;
            }
        } else {
            return Err("谱面类型无效".into());
        }
        self.command(Command::Load {
            path,
            title: file.name.clone(),
        })?;
        if kind == "notation" {
            self.command(Command::ActivateScore {
                id,
                content_id: cid,
            })?;
        } else {
            score_attachments::update(&self.data, &cid, Some(&id), "select", None, None, 0, 0)?;
        }
        self.command(Command::CurrentSong)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_score_import_repairs_owned_copy_and_merges_old_records() {
        let data = std::env::temp_dir().join(format!(
            "neothesia-score-copies-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut p = Player::new(data.clone(), PathBuf::new(), true);
        let xml =
            include_bytes!("../../neothesia-core/src/score_performance_test.musicxml").to_vec();
        for _ in 0..2 {
            p.command(Command::ImportScore {
                bytes: xml.clone(),
                name: "课本.musicxml".into(),
                default_bpm: 120,
            })
            .unwrap();
        }
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let path = p.file.as_ref().unwrap().source_path.clone().unwrap();
        assert_eq!(p.preferences.score_versions[&cid].len(), 1);
        let version = p.preferences.score_versions[&cid][0].clone();
        std::fs::write(&version.path, b"damaged").unwrap();
        p.command(Command::PairScore {
            bytes: xml.clone(),
            name: "课本.musicxml".into(),
            content_id: cid.clone(),
        })
        .unwrap();
        assert_eq!(p.preferences.score_versions[&cid].len(), 1);
        assert_eq!(p.preferences.score_versions[&cid][0].id, version.id);
        assert_eq!(std::fs::read(&version.path).unwrap(), xml);
        let mut copy = version.clone();
        copy.id = "old-copy".into();
        copy.path = data.join("scores/old-copy.musicxml");
        std::fs::copy(&version.path, &copy.path).unwrap();
        p.preferences
            .score_versions
            .get_mut(&cid)
            .unwrap()
            .push(copy.clone());
        p.preferences.save(&p.preferences_path).unwrap();
        metadata::save_score_association(&path, &cid, &copy.path).unwrap();
        assert_eq!(
            inspect(&data, &path, &cid).unwrap()["versions"][0]["copies"],
            2
        );
        p.manage_library_score(
            path.clone(),
            cid.clone(),
            "notation".into(),
            version.id.clone(),
            "deduplicate".into(),
            None,
        )
        .unwrap();
        assert_eq!(p.preferences.score_versions[&cid].len(), 1);
        assert!(copy.path.is_file());
        assert_eq!(
            sidecar(&path, &cid).unwrap().score.unwrap().path,
            version.path
        );
        let external = data.join("outside/teacher.musicxml");
        std::fs::create_dir_all(external.parent().unwrap()).unwrap();
        std::fs::write(&external, b"personal file changed").unwrap();
        p.preferences.score_versions.get_mut(&cid).unwrap()[0].path = external.clone();
        let score = notation::ScoreAsset::new(
            xml.clone(),
            "课本.musicxml".into(),
            p.file.as_ref().unwrap(),
        )
        .unwrap();
        let stored = p.store_score_version_file(&score, &cid).unwrap();
        assert_eq!(std::fs::read(&external).unwrap(), b"personal file changed");
        assert_eq!(std::fs::read(stored).unwrap(), xml);
        assert_eq!(p.preferences.score_versions[&cid][0].id, version.id);
    }
    #[test]
    fn noncurrent_versions_keep_playing_piece_and_reject_changed_files() {
        let data = std::env::temp_dir().join(format!(
            "neothesia-library-scores-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut p = Player::new(data.clone(), PathBuf::new(), true);
        let xml =
            include_bytes!("../../neothesia-core/src/score_performance_test.musicxml").to_vec();
        p.command(Command::ImportScore {
            bytes: xml.clone(),
            name: "原谱.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let path = p.file.as_ref().unwrap().source_path.clone().unwrap();
        p.command(Command::PairScore {
            bytes: xml,
            name: "教师版.musicxml".into(),
            content_id: cid.clone(),
        })
        .unwrap();
        let books = score_attachments::add(
            &data,
            &cid,
            "打印版.pdf",
            b"%PDF-1.4\nLibrary test".to_vec(),
            None,
        )
        .unwrap();
        let book = books["active"].as_str().unwrap().to_owned();
        let mut w = library_workspace::Workspace::load(&data).unwrap();
        w.inspect(path.clone(), true).unwrap();
        w.save(&data).unwrap();
        let listed = inspect(&data, &path, &cid).unwrap();
        assert_eq!(listed["versions"].as_array().unwrap().len(), 2);
        let active = listed["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["active"] == true)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let other = listed["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["active"] == false)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        p.command(Command::ImportMidi {
            bytes: include_bytes!("../assets/five-finger.mid").to_vec(),
            name: "正在练习.mid".into(),
        })
        .unwrap();
        let playing = p.file.as_ref().unwrap().content_id.clone();
        p.state.status = "playing".into();
        p.manage_library_score(
            path.clone(),
            cid.clone(),
            "notation".into(),
            other.clone(),
            "rename".into(),
            Some("课前版".into()),
        )
        .unwrap();
        assert_eq!(p.file.as_ref().unwrap().content_id, playing);
        assert_eq!(p.state.status, "playing");
        assert!(
            p.manage_library_score(
                path.clone(),
                cid.clone(),
                "notation".into(),
                active.clone(),
                "remove".into(),
                None
            )
            .is_err()
        );
        p.manage_library_score(
            path.clone(),
            cid.clone(),
            "notation".into(),
            active.clone(),
            "detach".into(),
            None,
        )
        .unwrap();
        p.manage_library_score(
            path.clone(),
            cid.clone(),
            "notation".into(),
            active,
            "remove".into(),
            None,
        )
        .unwrap();
        p.manage_library_score(
            path.clone(),
            cid.clone(),
            "notation".into(),
            other.clone(),
            "select".into(),
            None,
        )
        .unwrap();
        assert_eq!(p.state.status, "playing");
        p.manage_library_score(
            path.clone(),
            cid.clone(),
            "paper".into(),
            book.clone(),
            "rename".into(),
            Some("纸质课本".into()),
        )
        .unwrap();
        assert_eq!(p.file.as_ref().unwrap().content_id, playing);
        let v = inspect(&data, &path, &cid).unwrap();
        let score_path = PathBuf::from(v["versions"][0]["path"].as_str().unwrap());
        let original = std::fs::read(&score_path).unwrap();
        std::fs::write(&score_path, b"modified").unwrap();
        assert_eq!(
            inspect(&data, &path, &cid).unwrap()["versions"][0]["status"],
            "changed"
        );
        assert!(
            p.open_library_score(path.clone(), cid.clone(), "notation".into(), other.clone())
                .is_err()
        );
        assert_eq!(p.file.as_ref().unwrap().content_id, playing);
        std::fs::write(&score_path, original).unwrap();
        p.open_library_score(path.clone(), cid.clone(), "notation".into(), other.clone())
            .unwrap();
        assert_eq!(p.file.as_ref().unwrap().content_id, cid);
        std::fs::remove_file(&path).unwrap();
        let v = inspect(&data, &path, &cid).unwrap();
        assert_eq!(v["songAvailable"], false);
        assert!(
            p.open_library_score(path.clone(), cid.clone(), "paper".into(), book.clone())
                .is_err()
        );
        p.manage_library_score(
            path.clone(),
            cid.clone(),
            "paper".into(),
            book,
            "rename".into(),
            Some("缺失曲目的课本".into()),
        )
        .unwrap();
        std::fs::write(&path, include_bytes!("../assets/basic-chords.mid")).unwrap();
        assert!(
            p.manage_library_score(
                path,
                cid,
                "notation".into(),
                other,
                "rename".into(),
                Some("错误对象".into())
            )
            .is_err()
        );
    }
}
