use super::*;
use std::collections::BTreeSet;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    pub source_measure: usize,
    pub measure: usize,
    pub page: usize,
    #[serde(default)]
    pub region: Option<PageRegion>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub source_content_id: String,
    pub source: String,
    pub target: String,
    pub baseline: String,
    pub annotations: Vec<TransferAnnotation>,
    pub anchors: Vec<Anchor>,
}
fn copy_id(source_cid: &str, target_cid: &str, source: &str, target: &str, id: &str) -> String {
    blake3::hash(
        &serde_json::to_vec(&json!([
            "cross-paper-v1",
            source_cid,
            target_cid,
            source,
            target,
            id
        ]))
        .unwrap(),
    )
    .to_hex()
    .to_string()
}
fn baseline(
    source_cid: &str,
    target_cid: &str,
    source: &Attachment,
    target: &Attachment,
    grid: &str,
) -> String {
    blake3::hash(
        &serde_json::to_vec(&json!([source_cid, target_cid, source, target, grid])).unwrap(),
    )
    .to_hex()
    .to_string()
}
fn verify_assets(data: &Path, cid: &str, book: &Attachment) -> Result<(), String> {
    for asset in &book.pages {
        let p = asset_path(data, cid, asset)?;
        let meta = std::fs::metadata(&p).map_err(|_| "谱面文件缺失或不可读，请先修复谱面")?;
        if meta.len() != asset.size as u64 || meta.len() > MAX_BYTES as u64 {
            return Err("谱面文件已经改变，请重新导入后迁移资料".into());
        }
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        if blake3::hash(&bytes).to_hex().as_str() != asset.id {
            return Err("谱面文件已经改变，请重新导入后迁移资料".into());
        }
    }
    Ok(())
}
pub fn sources(data: &Path, target_cid: &str) -> Result<Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "谱面资料正在使用")?;
    let index = crate::library_workspace::Workspace::load(data)?;
    let root = data.join("score-attachments");
    let mut result = Vec::new();
    if !root.exists() {
        return Ok(json!({"sources":result}));
    }
    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path().join("manifest.json")) else {
            continue;
        };
        let Ok(store) = serde_json::from_slice::<Store>(&bytes) else {
            continue;
        };
        if store.content_id == target_cid
            || folder(data, &store.content_id).ok().as_ref() != Some(&entry.path())
        {
            continue;
        }
        let books:Vec<_>=store.attachments.iter().filter(|a|!a.annotations.is_empty()||a.mapping.as_ref().is_some_and(|m|!m.anchors.is_empty())).map(|a|json!({"id":a.id,"name":a.name,"format":a.format,"annotations":a.annotations.len(),"anchors":a.mapping.as_ref().map_or(0,|m|m.anchors.len())})).collect();
        if books.is_empty() {
            continue;
        }
        let known = index
            .entries
            .values()
            .find(|e| e.content_id.as_ref() == Some(&store.content_id));
        let title = known
            .and_then(|e| e.metadata.title.clone())
            .or_else(|| {
                known.and_then(|e| e.path.file_stem().map(|x| x.to_string_lossy().into_owned()))
            })
            .unwrap_or_else(|| {
                format!(
                    "{} · 演奏 {}",
                    store.attachments.first().map(|a|a.name.as_str()).unwrap_or("本机谱面"),
                    store.content_id.chars().take(8).collect::<String>()
                )
            });
        result.push(json!({"contentId":store.content_id,"title":title,"path":known.map(|e|&e.path),"attachments":books}));
    }
    result.sort_by(|a, b| a["title"].as_str().cmp(&b["title"].as_str()));
    Ok(json!({"sources":result}))
}
fn read_pair(
    data: &Path,
    target_cid: &str,
    source_cid: &str,
    source: &str,
    target: &str,
) -> Result<(Attachment, Store, usize), String> {
    if source_cid == target_cid {
        return Err("请选择另一份演奏；同一演奏的谱面请使用版本迁移".into());
    }
    let a = load(data, source_cid)?
        .attachments
        .into_iter()
        .find(|a| a.id == source)
        .ok_or("来源谱面不存在")?;
    let store = load(data, target_cid)?;
    let at = store
        .attachments
        .iter()
        .position(|a| a.id == target)
        .ok_or("目标谱面不存在")?;
    verify_assets(data, source_cid, &a)?;
    verify_assets(data, target_cid, &store.attachments[at])?;
    Ok((a, store, at))
}
impl crate::Player {
    fn cross_paper_grid(&self, cid: &str) -> Result<(String, Vec<Value>), String> {
        self.score_content_guard(cid)?;
        let loaded = self.file.as_ref().ok_or("请先打开目标演奏")?;
        let mut verified = if self.exercise.is_some() {
            let mut file = loaded.clone();
            file.musical_time = self.original_grid.clone().ok_or("原始小节网格不存在")?;
            file
        } else {
            crate::library::verified_file(&self.data, cid, loaded.source_path.clone().into_iter())?
        };
        let path = self.annotation_path(loaded)?;
        let sidecar = if neothesia_core::library::metadata_sidecar_path(&path).exists() {
            neothesia_core::library::load_song_sidecar(&path, cid).map_err(|e| e.to_string())?
        } else {
            Default::default()
        };
        verified.musical_time = Self::sidecar_grid(&verified, &sidecar)?;
        let grid = crate::practice_conditions::grid(&verified);
        if grid != self.grid_signature()
            || sidecar.meter_map != self.meter_map
            || sidecar.meter != self.meter_correction
        {
            return Err("目标小节设置已经改变，请重新打开目标演奏后迁移".into());
        }
        let measures=loaded.musical_time.measures.iter().map(|m|json!({"number":m.number,"displayNumber":m.display_number,"startTick":m.start_tick,"endTick":m.end_tick,"numerator":m.numerator,"denominator":m.denominator,"partial":m.partial,"explicit":m.explicit_meter,"start":loaded.tempo_track.pulses_to_duration(m.start_tick).as_secs_f64(),"end":loaded.tempo_track.pulses_to_duration(m.end_tick).as_secs_f64()})).collect();
        Ok((grid, measures))
    }
    pub(crate) fn preview_cross_paper(
        &self,
        cid: &str,
        source_cid: &str,
        source: &str,
        target: &str,
    ) -> Result<Value, String> {
        let lock = crate::library_workspace::lock_for(&self.data)?;
        let _guard = lock.lock().map_err(|_| "谱面资料正在使用")?;
        let (grid, measures) = self.cross_paper_grid(cid)?;
        let (a, store, at) = read_pair(&self.data, cid, source_cid, source, target)?;
        let b = &store.attachments[at];
        let imported: Vec<_> = a
            .annotations
            .iter()
            .filter(|n| {
                b.annotations
                    .iter()
                    .any(|t| t.id == copy_id(source_cid, cid, source, target, &n.id))
            })
            .map(|n| &n.id)
            .collect();
        let mut source_value = serde_json::to_value(&a).map_err(|e| e.to_string())?;
        let mut target_value = serde_json::to_value(b).map_err(|e| e.to_string())?;
        for book in [&mut source_value, &mut target_value] {
            for page in book["pages"].as_array_mut().unwrap() {
                page["available"] = json!(true);
                page["status"] = json!("ready");
            }
        }
        Ok(
            json!({"source":source_value,"target":target_value,"baseline":baseline(source_cid,cid,&a,b,&grid),"imported":imported,"targetGrid":grid,"targetMeasures":measures}),
        )
    }
    pub(crate) fn apply_cross_paper(&self, cid: &str, request: Request) -> Result<Value, String> {
        if request.annotations.len() > 2000
            || request.anchors.len() > 5000
            || (request.annotations.is_empty() && request.anchors.is_empty())
        {
            return Err("请选择需要迁移的批注或小节对应".into());
        }
        let lock = crate::library_workspace::lock_for(&self.data)?;
        let _guard = lock.lock().map_err(|_| "谱面资料正在使用")?;
        let (grid, measures) = self.cross_paper_grid(cid)?;
        let (source, mut store, at) = read_pair(
            &self.data,
            cid,
            &request.source_content_id,
            &request.source,
            &request.target,
        )?;
        let target = &mut store.attachments[at];
        if request.baseline != baseline(&request.source_content_id, cid, &source, target, &grid) {
            return Err("来源谱面、目标谱面或小节设置已经改变，请重新预览".into());
        }
        let mut seen = BTreeSet::new();
        let mut added = 0;
        for edit in &request.annotations {
            if !seen.insert(&edit.id) {
                return Err("迁移批注重复，资料未保存".into());
            }
            let note = source
                .annotations
                .iter()
                .find(|n| n.id == edit.id)
                .ok_or("来源批注已移除")?;
            let copy = Annotation {
                id: copy_id(
                    &request.source_content_id,
                    cid,
                    &request.source,
                    &request.target,
                    &edit.id,
                ),
                asset_id: edit.asset_id.clone(),
                page: edit.page,
                x: edit.x,
                y: edit.y,
                text: note.text.clone(),
                color: note.color.clone(),
                width: note.width,
                height: note.height,
                ink: note.ink.clone(),
            };
            if let Some(old) = target.annotations.iter().find(|n| n.id == copy.id) {
                if old != &copy {
                    return Err("这条批注已经迁移并修改，请在目标谱面继续编辑".into());
                }
            } else {
                target.annotations.push(copy);
                added += 1
            }
        }
        validate_annotations(
            &target.annotations,
            &target.format,
            &target
                .pages
                .iter()
                .map(|a| a.id.clone())
                .collect::<Vec<_>>(),
        )?;
        if !request.anchors.is_empty() {
            let source_map = source.mapping.as_ref().ok_or("来源谱面没有小节对应")?;
            let mut mapping = target.mapping.clone().unwrap_or(Mapping {
                grid: grid.clone(),
                enabled: false,
                anchors: vec![],
            });
            if mapping.grid != grid {
                return Err("目标谱面的旧小节对应已经过期，请先在目标谱面重新核对对应".into());
            }
            let mut source_seen = BTreeSet::new();
            let mut target_seen = BTreeSet::new();
            for edit in &request.anchors {
                if !source_seen.insert(edit.source_measure)
                    || !target_seen.insert(edit.measure)
                    || !source_map
                        .anchors
                        .iter()
                        .any(|a| a.measure == edit.source_measure)
                    || edit.measure == 0
                    || edit.measure > measures.len()
                {
                    return Err(
                        "请为每个来源小节明确选择一个有效的目标小节，同一小节不能重复".into(),
                    );
                }
                let new = PageAnchor {
                    measure: edit.measure,
                    page: edit.page,
                    region: edit.region.clone(),
                };
                if let Some(old) = mapping.anchors.iter().find(|a| a.measure == edit.measure) {
                    if old.page != new.page || old.region != new.region {
                        return Err(
                            "目标小节已有不同的谱面位置，请保留现有位置或取消这一项迁移".into()
                        );
                    }
                } else {
                    mapping.anchors.push(new)
                }
            }
            mapping.enabled=false;
            mapping.anchors.sort_by_key(|a| a.measure);
            mapping.validate(&target.format, target.pages.len())?;
            target.mapping = Some(mapping);
        }
        save(&self.data, &store)?;
        Ok(json!({"saved":true,"added":added,"anchors":request.anchors.len()}))
    }
}
