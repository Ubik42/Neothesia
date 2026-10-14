use crate::{
    library_workspace::Workspace,
    piece_package::{Asset, Manifest, Package, asset},
    preferences::{ScoreVersion, SessionSettings},
    *,
};
use neothesia_core::library::{
    ScoreAssociation, SongSidecar, load_song_sidecar, resolve_score_path, save_song_sidecar,
};
use std::collections::BTreeMap;

impl Player {
    pub(super) fn export_piece_package(
        &mut self,
        as_file: bool,
    ) -> Result<serde_json::Value, String> {
        self.remember_active_score()?;
        let midi: Vec<u8> =
            serde_json::from_value(self.command(Command::ExportMidi)?["bytes"].clone())
                .map_err(|e| e.to_string())?;
        let file = self.file.as_ref().ok_or("请先打开曲目")?;
        let id = file.content_id.clone();
        let path = self.annotation_path(file)?;
        let sidecar = read_sidecar(&path, &id)?;
        let mut files = BTreeMap::from([("performance.mid".into(), midi.clone())]);
        let midi_asset = asset(
            "performance.mid".into(),
            format!("{}.mid", self.title),
            &midi,
        );
        let mut scores: Vec<Asset> = vec![];
        let mut active_score = None;
        let active = sidecar.score.as_ref().map(|s| resolve_score_path(&path, s));
        for version in self
            .preferences
            .score_versions
            .get(&id)
            .into_iter()
            .flatten()
        {
            let bytes = std::fs::read(&version.path).map_err(|e| {
                format!("谱面 {} 无法打包：{e}，请先修复或移除该版本", version.name)
            })?;
            if blake3::hash(&bytes).to_hex().as_str() != version.content_id {
                return Err(format!("谱面 {} 已改变，请重新导入后打包", version.name));
            }
            let name = format!(
                "scores/{}.{}",
                scores.len(),
                if bytes.starts_with(b"PK") {
                    "mxl"
                } else {
                    "musicxml"
                }
            );
            if active.as_ref() == Some(&version.path) {
                active_score = Some(scores.len());
            }
            scores.push(asset(name.clone(), version.name.clone(), &bytes));
            files.insert(name, bytes);
        }
        let w = Workspace::load(&self.data)?;
        let entry = w
            .entries
            .values()
            .find(|e| e.content_id.as_ref() == Some(&id));
        let mut groups = vec![];
        for group in entry.into_iter().flat_map(|e| &e.groups) {
            let mut names = vec![];
            let mut current = Some(group.clone());
            while let Some(g) = current {
                if names.len() >= 20 {
                    return Err("分组层级过深，无法打包".into());
                }
                let row = w
                    .groups
                    .iter()
                    .find(|v| v.id == g)
                    .ok_or("曲目分组信息失效")?;
                names.push(row.name.clone());
                current = row.parent.clone();
            }
            names.reverse();
            groups.push(names);
        }
        let mut metadata = sidecar.metadata;
        if metadata.title.is_none() {
            metadata.title = Some(self.title.clone());
        }
        let root = std::env::var_os("NEOTHESIA_LIBRARY_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("D:/Music/MIDI/PracticeLibrary"));
        let origin = library::read(&root)
            .unwrap_or_default()
            .into_iter()
            .find(|r| file.source_path.as_ref() == Some(&r.path));
        if metadata.composer.is_none() {
            metadata.composer = origin
                .as_ref()
                .map(|r| r.composer.clone())
                .filter(|v| !v.is_empty());
        }
        let previous_origin = file
            .source_path
            .as_ref()
            .and_then(|p| p.parent())
            .and_then(|p| std::fs::read(p.join("manifest.json")).ok())
            .and_then(|b| serde_json::from_slice::<Manifest>(&b).ok())
            .map(|m| m.provenance);
        let provenance=origin.map(|r|serde_json::json!({"source":r.source,"category":r.category,"license":r.license,"sourceUrl":r.source_url})).or(previous_origin).unwrap_or_else(||serde_json::json!({"source":"个人曲目","originalContentId":id}));
        let fingers = file
            .tracks
            .iter()
            .flat_map(|t| {
                t.notes.iter().enumerate().filter_map(|(index, n)| {
                    self.fingers
                        .get(&(t.track_id, n.start, n.note))
                        .map(|finger| neothesia_core::library::FingerHint {
                            track_id: t.track_id,
                            note_index: index,
                            finger: *finger,
                        })
                })
            })
            .collect();
        let settings = SessionSettings {
            tracks: self.config.practice_track_setup(),
            parts: self
                .config
                .tracks
                .iter()
                .map(|t| (t.track_id, t.practice_part))
                .collect(),
            mode: Some(self.state.mode.clone()),
            count_in: self.state.count_in,
            metronome: self.state.metronome,
            latency: 0,
            rounds: self.state.rounds,
            hands: self.state.hands.clone(),
            adaptive: self.state.adaptive,
        };
        let mut hand_profiles = self
            .preferences
            .hand_span_profiles
            .get(&id)
            .cloned()
            .unwrap_or_default();
        for track in file.tracks.iter() {
            for hand in [PracticePart::LeftHand, PracticePart::RightHand] {
                let profile = self
                    .preferences
                    .hand_span_defaults
                    .get(&hand)
                    .copied()
                    .or_else(|| {
                        self.preferences
                            .fingering_profiles
                            .get(&id)
                            .and_then(|p| p.get(&track.track_id))
                            .copied()
                    })
                    .unwrap_or(neothesia_core::fingering::HandSpanProfile::Standard);
                hand_profiles
                    .entry(format!("{}:{}", track.track_id, hands::label(hand)))
                    .or_insert(profile);
            }
        }
        let (papers, active_paper) =
            score_attachments::export_package(&self.data, &id, &mut files)?;
        let mut ladders = self
            .preferences
            .ladder_presets
            .get(&id)
            .cloned()
            .unwrap_or_default();
        for l in &mut ladders {
            l.exercise_spec = None;
            l.settings.latency = 0;
        }
        let manifest = Manifest {
            format: "neothesia-piece".into(),
            version: if papers.iter().any(|p|!p.note_anchors.is_empty()){11}else if self.meter_map.is_some(){10}else if papers.iter().any(|p|p.annotations.iter().any(|n|n.ink.is_some()||n.width.is_some())||p.mapping.as_ref().is_some_and(|m|m.anchors.iter().any(|a|a.region.is_some()))){9}else if self.preferences.passages.get(&id).is_some_and(|p|p.iter().any(|p|p.score_range.as_ref().is_some_and(|r|r.start_trim>0||r.end_trim>0))){8}else if self.preferences.passages.get(&id).is_some_and(|p|p.iter().any(|p|p.precise_range.is_some())){7}else if !self.saved_actions()?.is_empty(){6}else if self.preferences.passages.get(&id).is_some_and(|p|p.iter().any(|p|p.score_range.is_some())){5}else if papers.iter().any(|p|!p.annotations.is_empty()){4}else if papers.iter().any(|p|p.mapping.is_some()){3}else{2},
            content_id: blake3::hash(&midi).to_hex().to_string(),
            title: self.title.clone(),
            metadata,
            midi: midi_asset,
            scores,
            active_score,
            papers,
            active_paper,
            ladders,
            fingerings: fingers,
            finger_actions:self.saved_actions()?,
            hands: self
                .note_hands
                .iter()
                .map(
                    |(&(track_id, note_index), &part)| neothesia_core::library::NoteHandHint {
                        track_id,
                        note_index,
                        part,
                    },
                )
                .collect(),
            track_parts: settings.parts.clone(),
            track_sounds:self.track_sound_settings(),
            track_appearances:self.track_appearances(),
            meter: self.meter_correction,
            meter_map:self.meter_map.clone(),
            passages: self
                .preferences
                .passages
                .get(&id)
                .cloned()
                .unwrap_or_default(),
            settings,
            speed: self.state.speed,
            practice_loop: self.state.passage.clone(),
            hand_profiles,
            profiles: self
                .preferences
                .fingering_profiles
                .get(&id)
                .cloned()
                .unwrap_or_default(),
            groups,
            rating: entry.map_or(0, |e| e.rating),
            provenance,
        };
        let package = Package { manifest, files };
        let summary = package.summary();
        let bytes = package.encode()?;
        if as_file {
            let directory = self.data.join("package-transfers");
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let path = directory.join(format!("{}.neopiece", blake3::hash(&bytes).to_hex()));
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({"path":path,"title":self.title,"summary":summary}))
        } else {
            Ok(serde_json::json!({"bytes":bytes,"title":self.title,"summary":summary}))
        }
    }
    pub(super) fn import_piece_package(
        &mut self,
        bytes: Vec<u8>,
        policy: String,
    ) -> Result<serde_json::Value, String> {
        if !["keep", "merge", "replace"].contains(&policy.as_str()) {
            return Err("请选择保留、合并或使用曲目包内容".into());
        }
        let package = Package::decode(&bytes)?;
        let m = &package.manifest;
        let id = &m.content_id;
        let mut w = Workspace::load(&self.data)?;
        let old_path = self
            .history
            .song(id)
            .and_then(|s| s.library.source_path.clone())
            .filter(|p| !p.is_file() || MidiFile::new(p).is_ok_and(|f| f.content_id == *id))
            .or_else(|| {
                w.entries
                    .values()
                    .find(|e| {
                        e.content_id.as_ref() == Some(id)
                            && (!e.path.is_file()
                                || MidiFile::new(&e.path).is_ok_and(|f| f.content_id == *id))
                    })
                    .map(|e| e.path.clone())
            });
        let old = old_path.as_ref().map(|p| read_sidecar(p, id)).transpose()?;
        let local_exists = old_path.is_some() || self.preferences.songs.contains_key(id);
        let directory = self
            .data
            .join("packages")
            .join(blake3::hash(&bytes).to_hex().as_str());
        let target = directory.join("performance.mid");
        let mut imported = SongSidecar {
            metadata: m.metadata.clone(),
            fingerings: m.fingerings.clone(),
            finger_actions:m.finger_actions.clone(),
            hands: m.hands.clone(),
            track_parts: m.track_parts.clone(),
            meter: m.meter,
            meter_map:m.meter_map.clone(),
            ..Default::default()
        };
        if let Some(index) = m.active_score {
            let a = &m.scores[index];
            imported.score = Some(ScoreAssociation {
                path: directory.join(&a.path),
                content_id: a.hash.clone(),
            });
        }
        let mut sidecar = if policy == "keep" {
            old.clone().unwrap_or_else(|| imported.clone())
        } else if policy == "merge" {
            if let Some(mut local) = old.clone() {
                let incoming =
                    serde_json::to_value(&imported.metadata).map_err(|e| e.to_string())?;
                let mut meta = serde_json::to_value(&local.metadata).map_err(|e| e.to_string())?;
                for (k, v) in incoming.as_object().unwrap() {
                    if meta[k].is_null() || meta[k].as_str() == Some("") {
                        meta[k] = v.clone();
                    }
                }
                local.metadata = serde_json::from_value(meta).map_err(|e| e.to_string())?;
                for tag in &imported.metadata.tags {
                    if !local.metadata.tags.contains(tag) {
                        local.metadata.tags.push(tag.clone());
                    }
                }
                for h in &imported.fingerings {
                    if !local
                        .fingerings
                        .iter()
                        .any(|l| l.track_id == h.track_id && l.note_index == h.note_index)
                    {
                        local.fingerings.push(*h);
                    }
                }
                for h in &imported.hands {
                    if !local
                        .hands
                        .iter()
                        .any(|l| l.track_id == h.track_id && l.note_index == h.note_index)
                    {
                        local.hands.push(*h);
                    }
                }
                // A held schedule is one reviewed unit; never splice two timelines.
                if local.finger_actions.is_empty(){local.finger_actions=imported.finger_actions.clone();}
                for (track, part) in &imported.track_parts {
                    local.track_parts.entry(*track).or_insert(*part);
                }
                if local.meter.is_none() && local.meter_map.is_none(){local.meter=imported.meter;local.meter_map=imported.meter_map;}
                if local.score.is_none() {
                    local.score = imported.score.clone();
                }
                local
            } else {
                imported.clone()
            }
        } else {
            imported
        };
        if let (Some(original), Some(score)) = (&old_path, &mut sidecar.score) {
            if old.as_ref().and_then(|s| s.score.as_ref()) == Some(score) {
                score.path = resolve_score_path(original, score);
            }
        }
        let mut preferences = self.preferences.clone();
        let imported_settings = !local_exists || policy == "replace";
        if imported_settings {
            preferences.songs.insert(id.clone(), m.settings.clone());
            preferences.track_sounds.insert(id.clone(),m.track_sounds.clone());
            preferences.track_appearances.insert(id.clone(),m.track_appearances.clone());
            preferences.passages.insert(id.clone(), m.passages.clone());
            preferences
                .hand_span_profiles
                .insert(id.clone(), m.hand_profiles.clone());
            preferences
                .fingering_profiles
                .insert(id.clone(), m.profiles.clone());
        } else if policy == "merge" {
            for (track,a) in &m.track_appearances{preferences.track_appearances.entry(id.clone()).or_default().entry(*track).or_insert_with(||a.clone());}
            for (track,sound) in &m.track_sounds{preferences.track_sounds.entry(id.clone()).or_default().entry(*track).or_insert_with(||sound.clone());}
            let passages = preferences.passages.entry(id.clone()).or_default();
            for p in &m.passages {
                if !passages.iter().any(|old| {
                    old.name == p.name
                        && old.start == p.start
                        && old.end == p.end
                        && old.notes == p.notes
                        && old.precise_range == p.precise_range
                        && old.score_range == p.score_range
                }) {
                    let mut p = p.clone();
                    p.id = format!(
                        "package-{}-{}",
                        &blake3::hash(&bytes).to_hex().as_str()[..12],
                        passages.len()
                    );
                    passages.push(p);
                }
            }
            for (key, value) in &m.hand_profiles {
                preferences
                    .hand_span_profiles
                    .entry(id.clone())
                    .or_default()
                    .entry(key.clone())
                    .or_insert(*value);
            }
            for (key, value) in &m.profiles {
                preferences
                    .fingering_profiles
                    .entry(id.clone())
                    .or_default()
                    .entry(*key)
                    .or_insert(*value);
            }
        }
        if imported_settings {
            preferences.ladder_presets.insert(
                id.clone(),
                m.ladders
                    .iter()
                    .cloned()
                    .map(|mut l| {
                        l.settings.latency = self.state.latency;
                        l
                    })
                    .collect(),
            );
            preferences
                .ladder_runs
                .retain(|key, _| !key.starts_with(&format!("{id}:")));
        } else if policy == "merge" {
            let ladders = preferences.ladder_presets.entry(id.clone()).or_default();
            for l in &m.ladders {
                if !ladders.iter().any(|old| {
                    old.name == l.name
                        && old.start == l.start
                        && old.end == l.end
                        && serde_json::to_value(&old.plan).ok()
                            == serde_json::to_value(&l.plan).ok()
                        && old.expression == l.expression
                        && {
                            let mut settings = old.settings.clone();
                            settings.latency = 0;
                            serde_json::to_value(&settings).ok()
                        } == {
                            let mut settings = l.settings.clone();
                            settings.latency = 0;
                            serde_json::to_value(&settings).ok()
                        }
                }) {
                    let mut l = l.clone();
                    l.settings.latency = self.state.latency;
                    l.id = format!(
                        "package-{}-{}",
                        &blake3::hash(&bytes).to_hex().as_str()[..12],
                        ladders.len()
                    );
                    ladders.push(l);
                }
            }
        }
        if preferences
            .ladder_presets
            .get(id)
            .is_some_and(|v| v.len() > 200)
        {
            return Err("合并后速度阶梯超过上限".into());
        }
        let versions = preferences.score_versions.entry(id.clone()).or_default();
        for (i, a) in m.scores.iter().enumerate() {
            if !versions
                .iter()
                .any(|v| v.content_id == a.hash && v.name == a.name)
            {
                versions.push(ScoreVersion {
                    id: format!("package-{}-{i}", a.hash),
                    name: a.name.clone(),
                    path: directory.join(&a.path),
                    content_id: a.hash.clone(),
                });
            }
        }
        if versions.len() > 100 || preferences.passages.get(id).is_some_and(|v| v.len() > 200) {
            return Err("合并后谱面或练习段超过数量上限，请整理后再导入".into());
        }
        if let Some(score) = &mut sidecar.score {
            if !versions.iter().any(|v| v.path == score.path) {
                if let Some(v) = versions
                    .iter()
                    .find(|v| v.content_id == score.content_id && v.path.is_file())
                {
                    score.path = v.path.clone();
                }
            }
        }
        // All merge limits above are checked before touching any user files. The
        // remaining writes form one recoverable transaction, including the files
        // changed by Load/persist and the paper attachment directory.
        let paper_dir = self.data.join("score-attachments").join(blake3::hash(id.as_bytes()).to_hex().as_str());
        let mut rollback = PackageImportRollback::new(&self.data, &[
            directory.clone(), paper_dir, self.preferences_path.clone(),
            self.data.join("library-workspace.json"),
            self.data.join("web-practice-history.ron"),
        ])?;
        let previous_preferences = self.preferences.clone();
        let previous_song = self.file.clone();
        let previous_title = self.title.clone();
        let previous_state = self.state.clone();
        let previous_config = self.config.clone();
        let mut loaded = false;
        let result = (|| -> Result<serde_json::Value, String> {
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        for (name, data) in &package.files {
            let path = directory.join(name);
            std::fs::create_dir_all(path.parent().ok_or("曲目包目录无效")?)
                .map_err(|e| e.to_string())?;
            std::fs::write(&path, data).map_err(|e| e.to_string())?;
        }
        score_attachments::import_package(
            &self.data,
            id,
            &m.papers,
            m.active_paper,
            &package.files,
            if imported_settings {
                "replace"
            } else {
                &policy
            },
        )?;
        save_song_sidecar(&target, id, sidecar).map_err(|e| e.to_string())?;
        preferences.save(&self.preferences_path)?;
        self.preferences = preferences;
        w.inspect(target.clone(), true)?;
        if policy != "keep" || !local_exists {
            if policy == "replace" {
                if let Some(e) = w.entries.get_mut(&library_workspace::key(&target)) {
                    e.groups.clear();
                }
            }
            for chain in &m.groups {
                let mut parent = None;
                for name in chain {
                    let found = w
                        .groups
                        .iter()
                        .find(|g| g.name == *name && g.parent == parent)
                        .map(|g| g.id.clone());
                    parent = Some(match found {
                        Some(id) => id,
                        None => w.group(None, name.clone(), parent.clone())?,
                    });
                }
                w.assign(vec![target.clone()], parent, None)?;
            }
            let rating = if policy == "merge" {
                w.entries
                    .get(&library_workspace::key(&target))
                    .map_or(m.rating, |e| e.rating.max(m.rating))
            } else {
                m.rating
            };
            w.assign(vec![target.clone()], None, Some(rating))?;
        }
        w.save(&self.data)?;
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec_pretty(m).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        loaded = true;
        self.command(Command::Load {
            path: target.clone(),
            title: m.title.clone(),
        })?;
        #[cfg(test)]
        if FAIL_PACKAGE_IMPORT_AFTER_LOAD.with(|flag| flag.get()) {
            return Err("test: import failed after Load".into());
        }
        if imported_settings {
            self.state.speed = m.speed;
            self.state.passage = m.practice_loop.clone();
            self.persist()?;
        }
        let mut result = self.command(Command::CurrentSong)?;
        result["packageResult"] = serde_json::json!({"policy":policy,"existing":local_exists,"scores":m.scores.len(),"passages":self.preferences.passages.get(id).map_or(0,|p|p.len())});
        Ok(result)
        })();
        match result {
            Ok(mut value) => {
                if let Err(error) = rollback.finish() {
                    value["packageResult"]["cleanupWarning"] = serde_json::json!(format!("导入已完成，但临时备份清理失败：{error}"));
                }
                Ok(value)
            }
            Err(error) => {
                let restore = rollback.restore();
                self.preferences = previous_preferences;
                self.history = PracticeHistoryStore::load(self.data.join("web-practice-history.ron"));
                let mut live_restore_error = None;
                if loaded {
                    if let Some(file) = previous_song {
                        let was_restoring = self.restoring;
                        self.restoring = true;
                        live_restore_error = self.install(file, previous_title, None).err();
                        self.restoring = was_restoring;
                    } else {
                        self.file = None;
                        self.title = previous_title;
                        self.notes.clear();
                        self.events.clear();
                        self.event_notes.clear();
                        self.note_hands.clear();
                        self.fingers.clear();
                        self.notation = None;
                        self.original_grid = None;
                        self.meter_map = None;
                        self.meter_correction = None;
                        self.config = previous_config;
                    }
                }
                self.state = previous_state;
                let mut rollback_errors = Vec::new();
                if let Err(restore_error) = restore { rollback_errors.push(restore_error); }
                if let Some(live_error) = live_restore_error { rollback_errors.push(format!("当前曲目恢复失败：{live_error}")); }
                if rollback_errors.is_empty() {
                    Err(error)
                } else {
                    Err(format!("{error}；导入回滚失败：{}", rollback_errors.join("；")))
                }
            }
        }
    }
}

struct PackageImportRollback {
    backup: PathBuf,
    paths: Vec<(PathBuf, bool)>,
}

impl PackageImportRollback {
    fn new(data: &std::path::Path, paths: &[PathBuf]) -> Result<Self, String> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
        for path in paths {
            let relative = path.strip_prefix(data).map_err(|_| "备份路径超出数据目录")?;
            if relative.components().any(|component| !matches!(component, std::path::Component::Normal(_))) {
                return Err("备份路径无效".into());
            }
            let mut ancestor = data.to_path_buf();
            for component in relative.components() {
                ancestor.push(component);
                if let Ok(metadata) = std::fs::symlink_metadata(&ancestor) {
                    reject_package_link(&metadata)?;
                }
            }
        }
        let backup = data.join(format!(
            ".package-import-rollback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&backup).map_err(|e| e.to_string())?;
        let mut transaction = Self { backup, paths: Vec::new() };
        for (index, path) in paths.iter().enumerate() {
            let exists = path.exists();
            if exists {
                if let Err(error) = copy_package_path(path, &transaction.backup.join(index.to_string())) {
                    let _ = std::fs::remove_dir_all(&transaction.backup);
                    return Err(error);
                }
            }
            transaction.paths.push((path.clone(), exists));
        }
        Ok(transaction)
    }

    fn restore(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();
        for (index, (path, existed)) in self.paths.iter().enumerate().rev() {
            let result = (|| -> Result<(), String> {
                remove_package_path(path)?;
                if *existed {
                    copy_package_path(&self.backup.join(index.to_string()), path)?;
                }
                Ok(())
            })();
            if let Err(error) = result { errors.push(error); }
        }
        if errors.is_empty() { self.finish() } else { Err(errors.join("；")) }
    }

    fn finish(&mut self) -> Result<(), String> {
        std::fs::remove_dir_all(&self.backup).map_err(|e| e.to_string())
    }
}

fn remove_package_path(path: &std::path::Path) -> Result<(), String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    verify_package_tree(path)?;
    if metadata.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
}

fn verify_package_tree(path: &std::path::Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    reject_package_link(&metadata)?;
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            verify_package_tree(&entry.map_err(|e| e.to_string())?.path())?;
        }
    }
    Ok(())
}

fn copy_package_path(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(from).map_err(|e| e.to_string())?;
    reject_package_link(&metadata)?;
    if from.is_dir() {
        std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
        for entry in std::fs::read_dir(from).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            copy_package_path(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        std::fs::create_dir_all(to.parent().ok_or("备份目录无效")?).map_err(|e| e.to_string())?;
        std::fs::copy(from, to).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn reject_package_link(metadata: &std::fs::Metadata) -> Result<(), String> {
    if metadata.file_type().is_symlink() { return Err("导入目录含有链接，无法安全备份".into()); }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("导入目录含有重解析点，无法安全备份".into());
        }
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static FAIL_PACKAGE_IMPORT_AFTER_LOAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
fn read_sidecar(path: &std::path::Path, id: &str) -> Result<SongSidecar, String> {
    match load_song_sidecar(path, id) {
        Ok(s) => Ok(s),
        Err(neothesia_core::library::MetadataError::Read(e))
            if e.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(SongSidecar::default())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn blank() -> Player {
        static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let data = std::env::temp_dir().join(format!(
            "neothesia-package-{}-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        Player::new(data, PathBuf::new(), true)
    }
    fn prepared() -> Player {
        let mut p = blank();
        p.preferences.hand_span_defaults.insert(
            PracticePart::LeftHand,
            neothesia_core::fingering::HandSpanProfile::Compact,
        );
        p.command(Command::ImportScore {
            bytes: include_bytes!("../../neothesia-core/src/score_performance_test.musicxml")
                .to_vec(),
            name: "练习谱.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        p.command(Command::NoteHand {
            track: 1,
            index: 0,
            part: Some(PracticePart::LeftHand),
        })
        .unwrap();
        p.command(Command::SavePassage {
            id: None,
            name: "连音练习".into(),
            start: 1,
            end: 2,
            notes: "注意保持".into(),
        })
        .unwrap();
        p.command(Command::Speed { value: 0.7 }).unwrap();
        let f = p.file.as_ref().unwrap();
        let path = p.annotation_path(f).unwrap();
        let id = f.content_id.clone();
        let mut annotations = read_sidecar(&path, &id).unwrap();
        annotations.metadata.composer = Some("教师整理".into());
        annotations.metadata.notes = Some("保持连奏".into());
        save_song_sidecar(&path, &id, annotations).unwrap();
        p.command(Command::PairScore {
            bytes: include_bytes!("../../neothesia-core/src/score_performance_test.musicxml")
                .to_vec(),
            name: "教师版.musicxml".into(),
            content_id: id.clone(),
        })
        .unwrap();
        let mut w = Workspace::load(&p.data).unwrap();
        let top = w.group(None, "教学".into(), None).unwrap();
        let child = w.group(None, "连音".into(), Some(top)).unwrap();
        w.inspect(path.clone(), true).unwrap();
        w.assign(vec![path], Some(child), Some(4)).unwrap();
        w.save(&p.data).unwrap();
        p
    }
    fn exported(p: &mut Player) -> Vec<u8> {
        serde_json::from_value(p.command(Command::ExportPackage).unwrap()["bytes"].clone()).unwrap()
    }
    #[test]
    fn failed_workspace_save_restores_imported_files_and_allows_retry() {
        let mut source = prepared();
        let cid = source.file.as_ref().unwrap().content_id.clone();
        score_attachments::add(
            &source.data,
            &cid,
            "练习纸谱",
            b"%PDF-1.4\nportable score".to_vec(),
            None,
        )
        .unwrap();
        let bytes = exported(&mut source);
        let mut target = blank();
        target.command(Command::ImportPackage { bytes: bytes.clone(), policy: "merge".into() }).unwrap();
        target.command(Command::NoteHand {
            track: 1,
            index: 0,
            part: Some(PracticePart::RightHand),
        }).unwrap();
        let package_dir = target.data.join("packages").join(blake3::hash(&bytes).to_hex().as_str());
        let paper_dir = target.data.join("score-attachments").join(blake3::hash(cid.as_bytes()).to_hex().as_str());
        let tracked = [
            neothesia_core::library::metadata_sidecar_path(&package_dir.join("performance.mid")),
            paper_dir.join("manifest.json"),
            target.preferences_path.clone(),
            target.data.join("library-workspace.json"),
            target.data.join("web-practice-history.ron"),
            package_dir.join("manifest.json"),
        ];
        let before: Vec<_> = tracked.iter().map(|path| std::fs::read(path).ok()).collect();
        let blocked_temp = target.data.join("library-workspace.json.tmp");
        std::fs::create_dir(&blocked_temp).unwrap();
        assert!(target.command(Command::ImportPackage { bytes: bytes.clone(), policy: "replace".into() }).is_err());
        let after: Vec<_> = tracked.iter().map(|path| std::fs::read(path).ok()).collect();
        assert_eq!(after, before);
        assert_eq!(target.note_hands.get(&(1, 0)), Some(&PracticePart::RightHand));
        assert_eq!(score_attachments::list(&target.data, &cid).unwrap()["attachments"].as_array().unwrap().len(), 1);
        std::fs::remove_dir(&blocked_temp).unwrap();
        target.command(Command::ImportPackage { bytes, policy: "replace".into() }).unwrap();
        assert_eq!(target.note_hands.get(&(1, 0)), Some(&PracticePart::LeftHand));
    }
    #[test]
    fn failure_after_load_restores_empty_player_and_allows_retry() {
        let mut source = prepared();
        let bytes = exported(&mut source);
        let mut target = blank();
        FAIL_PACKAGE_IMPORT_AFTER_LOAD.with(|flag| flag.set(true));
        let failed = target.command(Command::ImportPackage { bytes: bytes.clone(), policy: "merge".into() });
        FAIL_PACKAGE_IMPORT_AFTER_LOAD.with(|flag| flag.set(false));
        assert!(failed.is_err());
        assert!(target.file.is_none());
        assert!(target.notes.is_empty());
        assert!(target.preferences.songs.is_empty());
        assert!(!target.data.join("packages").join(blake3::hash(&bytes).to_hex().as_str()).exists());
        target.command(Command::ImportPackage { bytes, policy: "merge".into() }).unwrap();
        assert!(target.file.is_some());
    }
    #[test]
    fn failure_after_load_restores_previous_song() {
        let mut source = prepared();
        let bytes = exported(&mut source);
        let mut target = blank();
        target.command(Command::ImportPackage { bytes: bytes.clone(), policy: "merge".into() }).unwrap();
        target.command(Command::NoteHand { track: 1, index: 0, part: Some(PracticePart::RightHand) }).unwrap();
        let previous_path = target.file.as_ref().unwrap().source_path.clone();
        FAIL_PACKAGE_IMPORT_AFTER_LOAD.with(|flag| flag.set(true));
        let failed = target.command(Command::ImportPackage { bytes: bytes.clone(), policy: "replace".into() });
        FAIL_PACKAGE_IMPORT_AFTER_LOAD.with(|flag| flag.set(false));
        assert!(failed.is_err());
        assert_eq!(target.file.as_ref().unwrap().source_path, previous_path);
        assert_eq!(target.note_hands.get(&(1, 0)), Some(&PracticePart::RightHand));
        target.command(Command::ImportPackage { bytes, policy: "replace".into() }).unwrap();
        assert_eq!(target.note_hands.get(&(1, 0)), Some(&PracticePart::LeftHand));
    }
    #[test]
    fn package_note_anchor_v11_roundtrip_and_midi_identity_validation(){
        let mut source=prepared();
        let cid=source.file.as_ref().unwrap().content_id.clone();
        let paper=score_attachments::add(&source.data,&cid,"逐音练习谱",b"\x89PNG\r\n\x1a\nfirst".to_vec(),None).unwrap();
        let book=paper["active"].as_str().unwrap().to_string();
        let asset=paper["attachments"][0]["pages"][0]["id"].as_str().unwrap().to_string();
        let note=source.notes.first().unwrap();
        let track=note.track_id;let index=note.index;let pitch=note.note;
        let measure=source.file.as_ref().unwrap().measures.partition_point(|t|t.as_secs_f64()<=note.start.as_secs_f64());
        source.command(Command::EditPaperNoteAnchor{content_id:cid.clone(),book:book.clone(),track,index,draft:Some(score_attachments::NoteAnchorDraft{pitch,measure,asset_id:asset,page:1,region:score_attachments::PageRegion{x:0.3,y:0.4,width:0.05,height:0.05}}),expected:None}).unwrap();
        let bytes=exported(&mut source);
        let mut package=Package::decode(&bytes).unwrap();assert_eq!(package.manifest.version,11);
        let mut target=blank();target.command(Command::ImportPackage{bytes:bytes.clone(),policy:"merge".into()}).unwrap();
        let imported=score_attachments::list(&target.data,&cid).unwrap();
        assert_eq!(imported["attachments"][0]["noteAnchors"][0]["track"],track);
        assert_eq!(imported["attachments"][0]["noteAnchors"][0]["index"],index);
        target.command(Command::ImportPackage{bytes:bytes.clone(),policy:"merge".into()}).unwrap();
        let imported=score_attachments::list(&target.data,&cid).unwrap();
        assert_eq!(imported["attachments"][0]["noteAnchors"].as_array().unwrap().len(),1);
        let local_book=imported["attachments"][0]["id"].as_str().unwrap().to_string();
        let local_anchor:score_attachments::NoteAnchor=serde_json::from_value(imported["attachments"][0]["noteAnchors"][0].clone()).unwrap();
        score_attachments::edit_note_anchor(&target.data,&cid,&local_book,track,index,Some(score_attachments::NoteAnchorDraft{pitch,measure,asset_id:local_anchor.asset_id.clone(),page:1,region:score_attachments::PageRegion{x:0.5,y:0.4,width:0.05,height:0.05}}),Some(local_anchor),&target.grid_signature()).unwrap();
        assert!(target.command(Command::ImportPackage{bytes:bytes.clone(),policy:"merge".into()}).is_err());
        assert_eq!(score_attachments::list(&target.data,&cid).unwrap()["attachments"][0]["noteAnchors"][0]["region"]["x"],0.5);
        package.manifest.version=10;assert!(package.encode().is_err());
        package.manifest.version=11;package.manifest.papers[0].note_anchors[0].pitch=if pitch==60{61}else{60};assert!(package.encode().is_err());
    }
    #[test]
    fn package_ink_v9_roundtrip_and_older_manifest_refusal(){
        let mut source=prepared();let cid=source.file.as_ref().unwrap().content_id.clone();let paper=score_attachments::add(&source.data,&cid,"手写教学",b"\x89PNG\r\n\x1a\nink".to_vec(),None).unwrap();let book=paper["active"].as_str().unwrap();let asset=paper["attachments"][0]["pages"][0]["id"].as_str().unwrap();
        score_attachments::edit_ink(&source.data,&cid,book,asset,1,vec![],vec![score_attachments::Annotation{id:"ink-package".into(),asset_id:asset.into(),page:1,x:0.2,y:0.3,width:Some(0.2),height:Some(0.1),text:"教师连线".into(),color:"blue".into(),ink:Some(score_attachments::InkStroke{points:vec![[0.,0.],[1.,1.]],thickness:0.002})}]).unwrap();
        let bytes=exported(&mut source);let mut package=Package::decode(&bytes).unwrap();assert_eq!(package.manifest.version,9);let mut target=blank();target.command(Command::ImportPackage{bytes,policy:"merge".into()}).unwrap();let imported=score_attachments::list(&target.data,&cid).unwrap();assert_eq!(imported["attachments"][0]["annotations"][0]["ink"]["points"],serde_json::json!([[0.,0.],[1.,1.]]));
        package.manifest.version=8;assert!(package.encode().is_err());
        package.manifest.papers[0].annotations[0].ink=None;package.manifest.version=4;assert!(Package::decode(&package.encode().unwrap()).is_ok());
    }
    #[test]
    fn package_paper_ladder_portability_and_policy() {
        let mut source = prepared();
        let cid = source.file.as_ref().unwrap().content_id.clone();
        let pdf = b"%PDF-1.4\nportable score".to_vec();
        let page =
            score_attachments::add(&source.data, &cid, "教师谱.pdf".into(), pdf.clone(), None)
                .unwrap();
        score_attachments::update(
            &source.data,
            &cid,
            Some(page["active"].as_str().unwrap()),
            "view",
            None,
            Some(score_attachments::View {
                page: 3,
                zoom: 150,
                fit: false,
                rotation: 90,
            }),
            0,
            0,
        )
        .unwrap();
        source.command(Command::SetPaperMapping{content_id:cid.clone(),id:page["active"].as_str().unwrap().into(),mapping:Some(score_attachments::Mapping{grid:source.grid_signature(),enabled:true,anchors:vec![score_attachments::PageAnchor{measure:1,page:1,region:None},score_attachments::PageAnchor{measure:2,page:3,region:None}]})}).unwrap();
        let plan = neothesia_core::speed_ladder::SpeedLadderPlan {
            start_percent: 50,
            target_percent: 100,
            step_percent: 10,
            passes_required: 2,
            accuracy_percent: 90,
            on_time_percent: None,
            failures_before_step_back: 2,
            attempt_limit: 20,
        };
        source
            .command(Command::SaveLadder {
                id: None,
                name: "连音提速".into(),
                start: 1,
                end: 2,
                plan,
                expression: None,
            })
            .unwrap();
        let bytes = exported(&mut source);
        let package = Package::decode(&bytes).unwrap();
        assert_eq!(package.manifest.version, 3);
        source.command(Command::EditPaperAnnotation{content_id:cid.clone(),book:page["active"].as_str().unwrap().into(),id:None,expected:None,draft:Some(score_attachments::AnnotationDraft{asset_id:page["attachments"][0]["pages"][0]["id"].as_str().unwrap().into(),page:2,x:0.25,y:0.5,text:"踏板换在和声变化处".into(),color:"blue".into(),width:None,height:None,ink:None})}).unwrap();
        let bytes=exported(&mut source);let package=Package::decode(&bytes).unwrap();assert_eq!(package.manifest.version,4);
        assert_eq!(package.manifest.papers.len(), 1);
        assert_eq!(package.manifest.ladders.len(), 1);
        let mut target = blank();
        target
            .command(Command::Latency { milliseconds: 32 })
            .unwrap();
        target
            .command(Command::ImportPackage {
                bytes: bytes.clone(),
                policy: "merge".into(),
            })
            .unwrap();
        let cid = target.file.as_ref().unwrap().content_id.clone();
        let sheets = score_attachments::list(&target.data, &cid).unwrap();
        assert_eq!(sheets["attachments"][0]["annotations"][0]["text"],"踏板换在和声变化处");
        assert_eq!(sheets["attachments"][0]["mapping"]["anchors"][1]["page"],3);
        assert_eq!(sheets["attachments"][0]["mapping"]["grid"],target.grid_signature());
        assert_eq!(sheets["attachments"][0]["view"]["page"], 3);
        let id = sheets["active"].as_str().unwrap();
        assert_eq!(
            score_attachments::read(&target.data, &cid, id, 0)
                .unwrap()
                .0,
            pdf
        );
        score_attachments::update(
            &target.data,
            &cid,
            Some(id),
            "view",
            None,
            Some(score_attachments::View::default()),
            0,
            0,
        )
        .unwrap();
        target
            .command(Command::ImportPackage {
                bytes: bytes.clone(),
                policy: "merge".into(),
            })
            .unwrap();
        assert_eq!(
            score_attachments::list(&target.data, &cid).unwrap()["attachments"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            score_attachments::list(&target.data, &cid).unwrap()["attachments"][0]["view"]["page"],
            1
        );
        assert_eq!(target.preferences.ladder_presets[&cid].len(), 1);
        let lid = target.preferences.ladder_presets[&cid][0].id.clone();
        target
            .command(Command::StartLadder {
                id: lid,
                resume: false,
            })
            .unwrap();
        assert_eq!(target.state.latency, 32);
        target.command(Command::StopLadder).unwrap();
        target
            .command(Command::ImportPackage {
                bytes,
                policy: "replace".into(),
            })
            .unwrap();
        assert_eq!(
            score_attachments::list(&target.data, &cid).unwrap()["attachments"][0]["view"]["page"],
            3
        );
        let mut corrupted = package;
        corrupted.manifest.papers[0].view.zoom = 999;
        assert!(corrupted.encode().is_err());
        let mut legacy = Package::decode(&exported(&mut prepared())).unwrap();
        legacy.manifest.version = 1;
        assert!(Package::decode(&legacy.encode().unwrap()).is_ok());
    }
    #[test]
    fn package_roundtrip_restores_scores_annotations_passages_and_restart() {
        let mut source = prepared();
        let bytes = exported(&mut source);
        let mut target = blank();
        target
            .command(Command::ImportPackage {
                bytes: bytes.clone(),
                policy: "merge".into(),
            })
            .unwrap();
        let song = target.command(Command::CurrentSong).unwrap();
        assert_eq!(song["notes"].as_array().unwrap().len(), 6);
        assert_eq!(song["hasScore"], true);
        assert_eq!(target.state.speed, 0.7);
        assert_eq!(
            target.note_hands.get(&(1, 0)),
            Some(&PracticePart::LeftHand)
        );
        assert_eq!(
            target.command(Command::Passages).unwrap()["passages"][0]["name"],
            "连音练习"
        );
        let data = target.data.clone();
        drop(target);
        let mut restored = Player::new(data, PathBuf::new(), true);
        restored.restore();
        assert!(restored.notation.is_some());
        assert_eq!(
            restored.note_hands.get(&(1, 0)),
            Some(&PracticePart::LeftHand)
        );
        let passage = restored.preferences.passages.values().next().unwrap()[0]
            .id
            .clone();
        restored
            .command(Command::OpenPassage { id: passage })
            .unwrap();
        assert!(restored.state.passage.is_some());
        let again = exported(&mut restored);
        let parsed = Package::decode(&again).unwrap();
        assert_eq!(parsed.manifest.scores.len(), 2);
        assert_eq!(
            parsed.manifest.metadata.composer.as_deref(),
            Some("教师整理")
        );
        assert_eq!(parsed.manifest.rating, 4);
        assert_eq!(
            parsed.manifest.hand_profiles.get("1:left"),
            Some(&neothesia_core::fingering::HandSpanProfile::Compact)
        );
        assert_eq!(
            parsed.manifest.groups,
            vec![vec!["教学".to_string(), "连音".to_string()]]
        );
        assert_eq!(parsed.manifest.passages.len(), 1);
    }
    #[test]
    fn merge_keeps_local_conflicts_replace_uses_package_and_corruption_changes_nothing() {
        let mut source = prepared();
        let bytes = exported(&mut source);
        let mut target = blank();
        target
            .command(Command::ImportPackage {
                bytes: bytes.clone(),
                policy: "merge".into(),
            })
            .unwrap();
        target
            .command(Command::NoteHand {
                track: 1,
                index: 0,
                part: Some(PracticePart::RightHand),
            })
            .unwrap();
        let before = target.command(Command::CurrentSong).unwrap();
        assert!(
            target
                .command(Command::ImportPackage {
                    bytes: vec![1, 2, 3],
                    policy: "replace".into()
                })
                .is_err()
        );
        assert_eq!(
            target.command(Command::CurrentSong).unwrap()["notes"],
            before["notes"]
        );
        target
            .command(Command::ImportPackage {
                bytes: bytes.clone(),
                policy: "merge".into(),
            })
            .unwrap();
        assert_eq!(
            target.note_hands.get(&(1, 0)),
            Some(&PracticePart::RightHand)
        );
        target
            .command(Command::ImportPackage {
                bytes,
                policy: "replace".into(),
            })
            .unwrap();
        assert_eq!(
            target.note_hands.get(&(1, 0)),
            Some(&PracticePart::LeftHand)
        );
        assert_eq!(
            target.preferences.passages.values().next().unwrap().len(),
            1
        );
    }
    #[test]
    fn generated_exercise_exports_exact_midi_variant_with_its_fingers() {
        let mut p = blank();
        p.command(Command::Generate {
            spec: ExerciseSpec::default(),
        })
        .unwrap();
        let mut variant = ExerciseSpec::default();
        variant.repetitions = 2;
        variant.tempo_bpm = 80;
        p.command(Command::Generate { spec: variant }).unwrap();
        p.command(Command::SaveLadder {
            id: None,
            name: "固定练习提速".into(),
            start: 1,
            end: 2,
            plan: neothesia_core::speed_ladder::SpeedLadderPlan {
                start_percent: 50,
                target_percent: 100,
                step_percent: 10,
                passes_required: 2,
                accuracy_percent: 90,
                on_time_percent: None,
                failures_before_step_back: 2,
                attempt_limit: 20,
            },
            expression: None,
        })
        .unwrap();
        let expected_notes = p.notes.len();
        let expected_duration = p.state.duration;
        let count = p.fingers.len();
        let bytes = exported(&mut p);
        let parsed = Package::decode(&bytes).unwrap();
        assert_eq!(parsed.manifest.fingerings.len(), count);
        let mut target = blank();
        target
            .command(Command::ImportPackage {
                bytes,
                policy: "merge".into(),
            })
            .unwrap();
        let cid = target.file.as_ref().unwrap().content_id.clone();
        let lid = target.preferences.ladder_presets[&cid][0].id.clone();
        assert!(
            target.preferences.ladder_presets[&cid][0]
                .exercise_spec
                .is_none()
        );
        target
            .command(Command::StartLadder {
                id: lid,
                resume: false,
            })
            .unwrap();
        target.command(Command::StopLadder).unwrap();
        assert_eq!(target.fingers.len(), count);
        assert_eq!(target.notes.len(), expected_notes);
        assert!((target.state.duration - expected_duration).abs() < 0.00001);
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    #[test]
    fn file_hash_and_archive_paths_are_checked_before_import() {
        let mut p = super::tests::blank();
        p.command(Command::ImportMidi {
            bytes: include_bytes!("../assets/five-finger.mid").to_vec(),
            name: "练习.mid".into(),
        })
        .unwrap();
        let bytes: Vec<u8> =
            serde_json::from_value(p.command(Command::ExportPackage).unwrap()["bytes"].clone())
                .unwrap();
        let mut package = Package::decode(&bytes).unwrap();
        package.manifest.midi.hash = "wrong".into();
        assert!(package.encode().err().unwrap().contains("校验"));
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("../outside.mid", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut zip, b"x").unwrap();
        let bad = zip.finish().unwrap().into_inner();
        assert!(Package::decode(&bad).err().unwrap().contains("路径"));
    }
}
