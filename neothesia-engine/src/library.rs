use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongRow {
    pub id: String,
    pub title: String,
    pub composer: String,
    pub source: String,
    pub category: String,
    pub license: String,
    pub source_url: String,
    pub path: PathBuf,
    #[serde(default)]
    pub tags: Vec<String>,
    pub difficulty: Option<String>,
    pub rating: u8,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Catalog {
    title: String,
    composer: String,
    source: String,
    category: String,
    license: String,
    #[serde(default)]
    source_url: String,
    local_path: String,
}

pub fn read(root: &Path) -> Result<Vec<SongRow>, String> {
    let path = root.join("catalog.csv");
    let mut reader = csv::Reader::from_path(&path)
        .map_err(|e| format!("无法读取曲库目录 {}：{e}", path.display()))?;
    let mut rows = Vec::new();
    for row in reader.deserialize::<Catalog>().flatten() {
        let portable = row.local_path.replace('\\', "/");
        let relative = Path::new(&portable);
        if portable.contains(':')
            || relative.is_absolute()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            continue;
        }
        let path = root.join(relative);
        if !path.is_file() {
            continue;
        }
        rows.push(SongRow {
            id: portable,
            title: row.title,
            composer: row.composer,
            source: row.source,
            category: row.category,
            license: row.license,
            source_url: row.source_url,
            path,
            tags: vec![],
            difficulty: None,
            rating: 0,
        });
    }
    Ok(rows)
}

/// This original exercise is bundled, so first launch does not depend on an
/// external corpus. Third-party datasets continue to live outside the package.
pub fn load(root: &Path, data: &Path) -> Result<Vec<SongRow>, String> {
    std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let path = data.join("five-finger.mid");
    std::fs::write(&path, include_bytes!("../assets/five-finger.mid"))
        .map_err(|e| e.to_string())?;
    let mut rows = vec![SongRow {
        id: "builtin/five-finger".into(),
        title: "五指热身 · 从中央 C 开始".into(),
        composer: "Neothesia 原创练习".into(),
        source: "内置练习".into(),
        category: "入门/五指练习".into(),
        license: "随项目许可发布".into(),
        source_url: String::new(),
        path,
        tags: vec![],
        difficulty: None,
        rating: 0,
    }];
    for (name, title, category, bytes) in [
        (
            "c-major-scale",
            "C 大调音阶 · 一个八度",
            "入门/音阶",
            include_bytes!("../assets/c-major-scale.mid").as_slice(),
        ),
        (
            "bass-five-finger",
            "低音五指练习 · 左手音区",
            "入门/低音",
            include_bytes!("../assets/bass-five-finger.mid").as_slice(),
        ),
        (
            "parallel-five-finger",
            "双音同步 · 两个八度五指",
            "入门/双音",
            include_bytes!("../assets/parallel-five-finger.mid").as_slice(),
        ),
        (
            "basic-chords",
            "三和弦练习 · C / F / G",
            "入门/和弦",
            include_bytes!("../assets/basic-chords.mid").as_slice(),
        ),
    ] {
        let path = data.join(format!("{name}.mid"));
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        rows.push(SongRow {
            id: format!("builtin/{name}"),
            title: title.into(),
            composer: "Neothesia 原创练习".into(),
            source: "内置练习".into(),
            category: category.into(),
            license: "随项目许可发布".into(),
            source_url: String::new(),
            path,
            tags: vec![],
            difficulty: None,
            rating: 0,
        });
    }
    match read(root) {
        Ok(external) => rows.extend(external),
        Err(e) => eprintln!("{e}；仍可使用内置练习和导入 MIDI"),
    }
    match folders(data) {
        Ok(folders) => {
            for folder in folders {
                match scan_folder(&folder) {
                    Ok(found) => rows.extend(found),
                    Err(e) => eprintln!("曲库文件夹暂不可用 {}：{e}", folder.display()),
                }
            }
        }
        Err(e) => eprintln!("本地文件夹配置读取失败：{e}"),
    }

    let mut seen = std::collections::HashSet::new();
    rows.retain(|r| seen.insert(r.path.clone()));
    Ok(rows)
}

pub fn folders(data: &Path) -> Result<Vec<PathBuf>, String> {
    match std::fs::read(data.join("library-folders.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn folder_status(data: &Path) -> Result<serde_json::Value, String> {
    Ok(
        serde_json::json!({"folders":folders(data)?.into_iter().map(|path|serde_json::json!({"available":path.is_dir(),"path":path})).collect::<Vec<_>>()}),
    )
}
pub fn remove_folder(data: &Path, path: &Path) -> Result<(), String> {
    let mut roots = folders(data)?;
    if !roots.iter().any(|p| p == path) {
        return Err("该文件夹未登记".into());
    }
    roots.retain(|p| p != path);
    let temporary = data.join("library-folders.json.tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec(&roots).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temporary, data.join("library-folders.json")).map_err(|e| e.to_string())
}
pub fn command_paths(command: &crate::Command) -> Vec<PathBuf> {
    match command {
        crate::Command::Load { path, .. }
        | crate::Command::InspectSong { path, .. }
        | crate::Command::SetLibraryCollection { path, .. }
        | crate::Command::SaveSongLearning { path, .. }
        | crate::Command::LibraryMetadataHistory { path, .. } | crate::Command::PreviewMetadataRestore { path, .. } | crate::Command::RestoreLibraryMetadata { path, .. }
        | crate::Command::PreviewLibraryMetadata { path, .. }
        | crate::Command::ReadLibraryMetadata { path, .. }
        | crate::Command::SaveLibraryMetadata { path, .. }
        | crate::Command::UpdateSongMetadata { path, .. } => vec![path.clone()],
        crate::Command::IndexLibrary { paths, .. }
        | crate::Command::InspectSongs { paths, .. }
        | crate::Command::AssignLibrary { paths, .. }
        | crate::Command::UnassignLibrary { paths, .. } => paths.clone(),
        crate::Command::RelinkLibrary{items}=>items.iter().flat_map(|i|[i.from.clone(),i.to.clone()]).collect(),
        crate::Command::SetLibraryPrimary{path,..}=>path.clone().into_iter().collect(),
        crate::Command::RestoreLibraryPath{path,..}|crate::Command::IgnoreLibraryPath{path,..}=>vec![path.clone()],
        _ => vec![],
    }
}
pub fn add_folder(data: &Path, folder: PathBuf) -> Result<Vec<SongRow>, String> {
    let rows = scan_folder(&folder)?;
    let mut roots = folders(data)?;
    if !roots.contains(&folder) {
        roots.push(folder);
        let temporary = data.join("library-folders.json.tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec(&roots).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temporary, data.join("library-folders.json")).map_err(|e| e.to_string())?;
    }
    Ok(rows)
}
pub fn scan_folder(root: &Path) -> Result<Vec<SongRow>, String> {
    let mut rows = Vec::new();
    let mut pending = vec![(root.to_owned(), 0)];
    let mut visited = 0;
    while let Some((directory, depth)) = pending.pop() {
        visited += 1;
        if visited > 20_000 {
            return Err("文件夹数量过多，请选择更具体的曲库目录".into());
        }
        let entries = match std::fs::read_dir(&directory) {
            Ok(e) => e,
            Err(e) if depth == 0 => return Err(e.to_string()),
            Err(e) => return Err(format!("无法读取子目录 {}：{e}", directory.display())),
        };
        for entry in entries.flatten() {
            let kind = match entry.file_type() {
                Ok(k) => k,
                Err(_) => continue,
            };
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                if depth < 12 {
                    pending.push((path, depth + 1))
                }
                continue;
            }
            if !kind.is_file()
                || !path.extension().is_some_and(|e| {
                    e.eq_ignore_ascii_case("mid") || e.eq_ignore_ascii_case("midi")
                })
            {
                continue;
            }
            if rows.len() >= 50_000 {
                return Err("单个文件夹超过 50000 首，请分目录导入".into());
            }
            rows.push(SongRow {
                id: format!("folder/{}", path.display()),
                title: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                composer: String::new(),
                source: "本地文件夹".into(),
                category: root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                license: "本地文件，请确认使用权限".into(),
                source_url: String::new(),
                path,
                tags: vec![],
                difficulty: None,
                rating: 0,
            });
        }
    }
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(rows)
}
pub fn enriched(rows: &[SongRow], data: &Path) -> Vec<SongRow> {
    let mut rows = rows.to_vec();
    if let Ok(workspace) = crate::library_workspace::Workspace::load(data) {
        let mut known: std::collections::HashSet<_> = rows
            .iter()
            .map(|r| crate::library_workspace::key(&r.path))
            .collect();
        for entry in workspace.entries.values() {
            if known.insert(crate::library_workspace::key(&entry.path)) {
                rows.push(SongRow {
                    id: format!("indexed/{}", entry.path.display()),
                    title: entry.metadata.title.clone().unwrap_or_else(|| {
                        entry
                            .path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into()
                    }),
                    composer: entry.metadata.composer.clone().unwrap_or_default(),
                    source: "本地索引".into(),
                    category: entry.metadata.collection.clone().unwrap_or_default(),
                    license: "本地文件，请确认使用权限".into(),
                    source_url: String::new(),
                    path: entry.path.clone(),
                    tags: vec![],
                    difficulty: None,
                    rating: 0,
                });
            }
        }
        rows.retain(|row|workspace.entries.get(&crate::library_workspace::key(&row.path)).and_then(|e|e.content_id.as_ref()).is_none_or(|id|crate::library_repairs::link_target(&workspace,id,&row.path).is_none() && !workspace.ignored_paths.contains(&crate::library_repairs::record_key(id,&row.path))));
        for row in &mut rows {
            if let Some(entry) = workspace
                .entries
                .get(&crate::library_workspace::key(&row.path))
            {
                if let Some(title) = &entry.metadata.title {
                    if !title.is_empty() {
                        row.title = title.clone();
                    }
                }
                if let Some(composer) = &entry.metadata.composer {
                    row.composer = composer.clone();
                }
                row.tags = entry.metadata.tags.clone();
                row.difficulty = entry.metadata.difficulty.clone();
                row.rating = entry.rating;
            }
        }
    }
    rows
}
/// Recover the portable annotations of an identical MIDI at a new location.
/// Existing valid annotations at the destination always win.
pub fn recover_annotations(
    original: &Path,
    destination: &Path,
    content_id: &str,
) -> Result<(), String> {
    match neothesia_core::library::load_song_sidecar(destination, content_id) {
        Ok(_) => return Ok(()),
        Err(neothesia_core::library::MetadataError::Read(e))
            if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("新位置已有无法合并的附加信息：{e}")),
    }
    let mut old = match neothesia_core::library::load_song_sidecar(original, content_id) {
        Ok(s) => s,
        Err(neothesia_core::library::MetadataError::Read(e))
            if e.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(());
        }
        Err(e) => return Err(e.to_string()),
    };
    if let Some(score) = old.score.as_ref() {
        if neothesia_core::library::verify_score_association(original, score)
            .is_ok_and(|valid| valid)
        {
            let path = neothesia_core::library::resolve_score_path(original, score);
            old.score = Some(neothesia_core::library::ScoreAssociation {
                path,
                content_id: score.content_id.clone(),
            });
        } else {
            old.score = None;
            old.score_analysis = None;
        }
    }
    neothesia_core::library::save_song_sidecar(destination, content_id, old)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn validate_paths(
    paths: Vec<PathBuf>,
    rows: &[SongRow],
    known: &serde_json::Value,
    indexed: &serde_json::Value,
) -> Result<(), String> {
    let mut allowed: std::collections::HashSet<_> = rows
        .iter()
        .map(|r| crate::library_workspace::key(&r.path))
        .collect();
    for v in known["songs"].as_array().into_iter().flatten() {
        for p in [v["path"].as_str(),v["storedPath"].as_str()].into_iter().flatten(){allowed.insert(crate::library_workspace::key(Path::new(p)));}
    }
    for v in indexed["entries"]
        .as_object()
        .into_iter()
        .flat_map(|v| v.values())
    {
        if let Some(p) = v["path"].as_str() {
            allowed.insert(crate::library_workspace::key(Path::new(p)));
        }
    }
    if paths
        .iter()
        .any(|p| !allowed.contains(&crate::library_workspace::key(p)))
    {
        return Err("文件不在已登记的曲库中，请先导入".into());
    }
    Ok(())
}

/// A known content identity may have moved to another indexed path.
pub fn available_path(index:&crate::library_workspace::Workspace,content_id:&str,preferred:Option<&Path>)->Option<PathBuf>{
 if let Some(path)=index.preferred_paths.get(content_id){if path.is_file()&&index.entries.get(&crate::library_workspace::key(path)).is_some_and(|e|e.content_id.as_deref()==Some(content_id)){return Some(path.clone());}}
 if let Some(path)=preferred {if path.is_file() && index.entries.get(&crate::library_workspace::key(path)).is_none_or(|e|e.content_id.as_deref()==Some(content_id)){return Some(path.to_owned());}}
 index.entries.values().filter(|e|e.content_id.as_deref()==Some(content_id)&&e.path.is_file()).find(|e|crate::library_repairs::link_target(index,content_id,&e.path).is_none()).or_else(||index.entries.values().find(|e|e.content_id.as_deref()==Some(content_id)&&e.path.is_file())).map(|e|e.path.clone())
}
pub fn verified_file(data:&Path,content_id:&str,preferred:impl IntoIterator<Item=PathBuf>)->Result<midi_file::MidiFile,String>{
 let index=crate::library_workspace::Workspace::load(data)?;
 let mut paths=available_path(&index,content_id,None).into_iter().chain(preferred).collect::<Vec<_>>();
 paths.extend(index.entries.values().filter(|e|e.content_id.as_deref()==Some(content_id)).map(|e|e.path.clone()));
 let mut seen=std::collections::HashSet::new();
 for path in paths {if seen.insert(crate::library_workspace::key(&path)){if let Ok(file)=midi_file::MidiFile::new(&path){if file.content_id==content_id{return Ok(file);}}}}
 Err("曲目文件缺失、内容改变或无法读取，请定位同一内容的文件后重试".into())
}
