#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod score_batch_export;
mod practice_backup_bridge;
use neothesia_engine::{Command, Engine, library};
use std::path::PathBuf;
use tauri::{Manager,Emitter};

struct State {
    engine: Engine,
    songs: std::sync::Arc<std::sync::Mutex<Vec<library::SongRow>>>,
    monitor: neothesia_engine::library_monitor::Monitor,
    data: PathBuf,
    font: PathBuf,
    bundled_font: PathBuf,
    soundfont_status: std::sync::Arc<std::sync::Mutex<SoundFontStatus>>,
    soundfont_change: std::sync::Arc<std::sync::Mutex<()>>,
    score_imports: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String,std::sync::Arc<std::sync::atomic::AtomicBool>>>>,
    generated_scores: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String,GeneratedScorePreview>>>,
    located_paper_transfer: std::sync::Mutex<Option<LocatedPaperTransfer>>,
    _data_lock: std::fs::File,
}
struct LocatedPaperTransfer { id:String, size:usize, bytes:Vec<u8> }

#[derive(Clone,serde::Serialize)]
#[serde(rename_all="camelCase")]
struct SoundFontStatus {
    path: Option<String>,
    label: String,
    is_default: bool,
    warning: Option<String>,
}

fn soundfont_config(data:&std::path::Path)->PathBuf {data.join("soundfont.json")}

fn save_soundfont(data:&std::path::Path,path:&std::path::Path)->Result<(),String>{
    use std::io::Write;
    let target=soundfont_config(data);
    let temporary=target.with_extension("json.tmp");
    let bytes=serde_json::to_vec_pretty(&serde_json::json!({"path":path})).map_err(|e|e.to_string())?;
    let mut file=std::fs::File::create(&temporary).map_err(|e|e.to_string())?;
    file.write_all(&bytes).map_err(|e|e.to_string())?;
    file.sync_all().map_err(|e|e.to_string())?;
    drop(file);
    std::fs::rename(&temporary,&target).map_err(|e|format!("无法写入音源设置：{e}"))
}

fn saved_soundfont(data:&std::path::Path)->(Option<PathBuf>,Option<String>){
    let file=soundfont_config(data);
    if !file.exists(){return (None,None)}
    let value=std::fs::read(&file).ok().and_then(|bytes|serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let Some(path)=value.as_ref().and_then(|value|value["path"].as_str()).map(PathBuf::from) else {
        return (None,Some("已保存的音源设置无效，已使用随包钢琴音源".into()))
    };
    if !path.is_absolute(){return (None,Some("已保存的音源路径不是绝对路径，已使用随包钢琴音源".into()))}
    if !path.is_file(){return (None,Some(format!("上次使用的音源已找不到：{}；已使用随包钢琴音源",path.display())))}
    match neothesia_engine::validate_soundfont(&path){
        Ok(())=>(Some(path),None),
        Err(error)=>(None,Some(format!("上次使用的音源无法加载：{error}；已使用随包钢琴音源"))),
    }
}

fn soundfont_status(path:&std::path::Path,bundled:&std::path::Path,warning:Option<String>)->SoundFontStatus{
    let is_default=path==bundled;
    SoundFontStatus{
        path:if is_default{None}else{Some(path.to_string_lossy().to_string())},
        label:if is_default{"随包钢琴音源".into()}else{path.file_name().unwrap_or_default().to_string_lossy().to_string()},
        is_default,
        warning,
    }
}

#[tauri::command]
fn get_soundfont_status(state:tauri::State<'_,State>)->Result<SoundFontStatus,String>{
    state.soundfont_status.lock().map(|status|status.clone()).map_err(|e|e.to_string())
}

#[tauri::command]
async fn select_soundfont(state:tauri::State<'_,State>)->Result<Option<SoundFontStatus>,String>{
    let Some(file)=file_dialog(&state.data,"soundfont").set_title("选择本机 SoundFont 音源").add_filter("SoundFont 音源", &["sf2"]).pick_file().await else{return Ok(None)};
    let path=file.path().canonicalize().map_err(|e|format!("无法读取音源文件：{e}"))?;
    if !path.extension().and_then(|extension|extension.to_str()).is_some_and(|extension|extension.eq_ignore_ascii_case("sf2")){return Err("请选择 .sf2 音源文件".into())}
    let engine=state.engine.clone();let data=state.data.clone();let bundled=state.bundled_font.clone();
    let status=state.soundfont_status.clone();let change=state.soundfont_change.clone();
    let selected=tauri::async_runtime::spawn_blocking(move||{
        let _guard=change.lock().map_err(|e|e.to_string())?;
        let previous=status.lock().map_err(|e|e.to_string())?.clone();
        engine.command(Command::SoundFont{path:path.clone()})?;
        if let Err(error)=save_soundfont(&data,&path){
            let restore=previous.path.as_ref().map(PathBuf::from).unwrap_or(bundled.clone());
            let _=engine.command(Command::SoundFont{path:restore});
            return Err(format!("音源设置保存失败：{error}"))
        }
        remember_location(&data,"soundfont",&path,false);
        let current=soundfont_status(&path,&bundled,None);
        *status.lock().map_err(|e|e.to_string())?=current.clone();
        Ok::<_,String>(current)
    }).await.map_err(|e|e.to_string())??;
    Ok(Some(selected))
}

#[tauri::command]
async fn reset_soundfont(state:tauri::State<'_,State>)->Result<SoundFontStatus,String>{
    let engine=state.engine.clone();let bundled=state.bundled_font.clone();let data=state.data.clone();
    let status=state.soundfont_status.clone();let change=state.soundfont_change.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let _guard=change.lock().map_err(|e|e.to_string())?;
        let previous=status.lock().map_err(|e|e.to_string())?.clone();
        engine.command(Command::SoundFont{path:bundled.clone()})?;
        let config=soundfont_config(&data);
        if let Err(error)=std::fs::remove_file(&config){if error.kind()!=std::io::ErrorKind::NotFound{
            let restore=previous.path.as_ref().map(PathBuf::from).unwrap_or(bundled.clone());
            let _=engine.command(Command::SoundFont{path:restore});
            return Err(format!("恢复钢琴音源后无法保存设置：{error}"))
        }}
        let current=soundfont_status(&bundled,&bundled,None);
        *status.lock().map_err(|e|e.to_string())?=current.clone();
        Ok::<_,String>(current)
    }).await.map_err(|e|e.to_string())?
}

#[derive(Clone)]
struct GeneratedScorePreview {
    content_id: String,
    source: PathBuf,
    name: String,
    hash: String,
}

fn file_dialog(data:&std::path::Path,kind:&str)->rfd::AsyncFileDialog{let dialog=rfd::AsyncFileDialog::new();if let Some(path)=neothesia_engine::file_locations::initial(data,kind){dialog.set_directory(path)}else{dialog}}
fn remember_location(data:&std::path::Path,kind:&str,path:&std::path::Path,folder:bool){if let Err(error)=neothesia_engine::file_locations::remember(data,kind,path,folder){eprintln!("{error}");}}

fn headless_check() -> bool {
    std::env::var_os("NEOTHESIA_HEADLESS_CHECK").is_some()
        || std::env::args().any(|a| a == "--headless-check")
}
fn startup_check() -> bool {
    headless_check() || std::env::args().any(|a| a == "--window-check")
}
fn startup_stage(label: &str) {
    if !startup_check() { return; }
    let Some(path)=std::env::var_os("NEOTHESIA_STARTUP_CHECK_LOG") else { return; };
    if let Ok(mut file)=std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _=writeln!(file,"stage={label}");
    }
}

#[tauri::command]
async fn reveal_library_file(state:tauri::State<'_,State>,path:PathBuf)->Result<(),String>{
    let known=state.engine.command(Command::Collection)?;let indexed=state.engine.command(Command::LibraryWorkspace)?;
    {let rows=state.songs.lock().map_err(|e|e.to_string())?;library::validate_paths(vec![path.clone()],&rows,&known,&indexed)?;}
    tauri::async_runtime::spawn_blocking(move||{
        let requested_file=path.is_file();
        let mut target=path.clone();
        if !requested_file {while !target.is_dir(){if !target.pop(){return Err("原文件和上级文件夹均不可用".into());}}}
        let target=target.canonicalize().map_err(|e|format!("无法打开文件位置：{e}"))?;
        #[cfg(target_os="windows")]
        {let text=target.to_string_lossy();let text=if let Some(unc)=text.strip_prefix(r"\\?\UNC\"){format!(r"\\{unc}")}else{text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()};let arg=if requested_file{format!("/select,{text}")}else{text.to_string()};std::process::Command::new("explorer.exe").arg(arg).spawn().map_err(|e|format!("无法打开文件位置：{e}"))?;}
        #[cfg(target_os="macos")]
        {let mut command=std::process::Command::new("open");if requested_file{command.arg("-R");}command.arg(&target).spawn().map_err(|e|e.to_string())?;}
        #[cfg(all(not(target_os="windows"),not(target_os="macos")))]
        {let directory=if requested_file{target.parent().ok_or("文件上级位置无效")?}else{target.as_path()};std::process::Command::new("xdg-open").arg(directory).spawn().map_err(|e|e.to_string())?;}
        Ok(())
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn native_startup_check(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    ready: bool,
    notation_ready: bool,
    pdf_ready:bool,
) -> Result<(), String> {
    if !startup_check() {
        return Err("启动检查仅在隐藏验证模式启用".into());
    }
    startup_stage("native_check_called");
    if std::env::args().any(|a| a == "--window-check") {
        let window = app.get_webview_window("main").ok_or("主窗口未创建")?;
        if !window.is_visible().map_err(|e| e.to_string())? {
            startup_stage("window_not_visible");
            return Err("主窗口没有显示".into());
        }
        startup_stage("window_visible");
    }
    let engine = state.engine.clone();
    let row = state
        .songs
        .lock()
        .map_err(|e| e.to_string())?
        .first()
        .cloned()
        .ok_or("内置练习缺失")?;
    let count = state.songs.lock().map_err(|e| e.to_string())?.len();
    let font = state.font.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        startup_stage("native_worker_started");
        if !pdf_ready {return Err("PDF 谱页渲染器未通过桌面检查".into());}
        if !notation_ready {return Err("桌面乐谱 WASM 渲染器未通过启动检查".into());}
        if !ready { return Err("桌面前端未完成真实引擎连接".into()); }
        startup_stage("frontend_evidence_ready");
        let song = engine.command(Command::Load { path: row.path, title: row.title })?;
        startup_stage("song_loaded");
        if song["notes"].as_array().map(Vec::len) != Some(9) { return Err("内置练习解析失败".into()); }
        engine.command(Command::Speed { value: 0.5 })?;
        startup_stage("speed_set");
        neothesia_engine::probe_audio(&font)?;
        startup_stage("audio_ready");
        if let Some(path) = std::env::var_os("NEOTHESIA_STARTUP_CHECK_LOG") {
            std::fs::write(path, format!("PASS\nvisible={}\nfrontend=ready\nIPC=ready\nlibraryRows={count}\nnotes=9\naudio=ready\nnotationWasm=ready\npdfPages=ready\nmidiConnected={}\n", !headless_check(),engine.snapshot().input_connected)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }).await.unwrap_or_else(|error|Err(format!("启动检查线程未完成：{error}")));
    match result {
        Ok(()) => app.exit(0),
        Err(e) => {
            if let Some(path) = std::env::var_os("NEOTHESIA_STARTUP_CHECK_LOG") {
                let stages=std::fs::read_to_string(&path).unwrap_or_default();
                let _ = std::fs::write(path, format!("FAILED\n{e}\n{stages}"));
            }
            app.exit(1);
        }
    }
    Ok(())
}

#[tauri::command]
async fn score_attachment_bytes(state:tauri::State<'_,State>,content_id:String,id:String,page:usize,preview_kind:Option<String>)->Result<tauri::ipc::Response,String>{
 let data=state.data.clone();tauri::async_runtime::spawn_blocking(move||(if let Some(kind)=preview_kind{neothesia_engine::score_attachments::read_source_preview(&data,&content_id,&id,&kind)}else{neothesia_engine::score_attachments::read(&data,&content_id,&id,page)}).map(|(bytes,_)|tauri::ipc::Response::new(bytes))).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn pick_paper_source(state:tauri::State<'_,State>)->Result<Option<String>,String>{
 let Some(file)=file_dialog(&state.data,"paper").set_title("选择内容相同的谱页原件").add_filter("谱页", &["pdf","png","jpg","jpeg","webp"]).pick_file().await else{return Ok(None)};
 remember_location(&state.data,"paper",file.path(),false);Ok(Some(file.path().to_string_lossy().into()))
}
fn link_imported_paper_source(engine:&neothesia_engine::Engine,cid:&str,value:&serde_json::Value,path:PathBuf,hash:&str)->Result<(),String>{
 let book=value["active"].as_str().ok_or("谱面版本编号缺失")?;
 engine.command(Command::LinkPaperSource{content_id:cid.into(),book:book.into(),asset:hash.into(),path:Some(path),baseline:None}).map(|_|()).map_err(|e|format!("谱面已保存，原件关联失败：{e}"))
}
#[tauri::command]
async fn import_score_attachments(state:tauri::State<'_,State>,content_id:String,append:Option<String>)->Result<Option<serde_json::Value>,String>{
 let Some(files)=file_dialog(&state.data,"paper").set_title("添加 PDF 或图片谱面").add_filter("谱面", &["pdf","png","jpg","jpeg","webp"]).pick_files().await else{return Ok(None)};
 for file in &files{remember_location(&state.data,"paper",file.path(),false);}
 let paths=files.into_iter().map(|f|f.path().to_path_buf()).collect::<Vec<_>>();let engine=state.engine.clone();
 tauri::async_runtime::spawn_blocking(move||{let mut results=Vec::new();let only_append=append.is_some();let mut append_to=append;for path in paths{let result=(||->Result<serde_json::Value,String>{if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>48_000_000{return Err("谱面超过 48 MB".into())}let bytes=std::fs::read(&path).map_err(|e|e.to_string())?;let image=!bytes.starts_with(b"%PDF-");let hash=blake3::hash(&bytes).to_hex().to_string();let value=engine.command(Command::AddScoreAttachment{content_id:content_id.clone(),name:path.file_name().unwrap_or_default().to_string_lossy().into(),bytes,append:if image||only_append{append_to.clone()}else{None}})?;link_imported_paper_source(&engine,&content_id,&value,path.clone(),&hash)?;if image&&append_to.is_none(){append_to=value["active"].as_str().map(String::from);}Ok(value)})();results.push(match result{Ok(_)=>serde_json::json!({"name":path.file_name().unwrap_or_default().to_string_lossy(),"ok":true}),Err(e)=>serde_json::json!({"name":path.file_name().unwrap_or_default().to_string_lossy(),"ok":false,"error":e})});}Ok(Some(serde_json::json!({"results":results}))) }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn import_library_scores(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    batch_id: String,
    path: String,
    content_id: String,
    kind: String,
    target: Option<String>,
    page: usize,
) -> Result<Option<serde_json::Value>, String> {
    let location_kind=if kind=="repairPaper"||kind=="notation"&&target.is_some(){"repair"}else if kind=="notation"{"notation"}else{"paper"};
    let dialog = file_dialog(&state.data,location_kind).set_title("为曲库曲目添加或修复谱面");
    let dialog = if kind == "notation" {
        dialog.add_filter("演奏乐谱", &["musicxml", "xml", "mxl"])
    } else if kind == "append" {
        dialog.add_filter("图片谱页", &["png", "jpg", "jpeg", "webp"])
    } else {
        dialog.add_filter("谱页", &["pdf", "png", "jpg", "jpeg", "webp"])
    };
    let Some(files) = dialog.pick_files().await else {
        return Ok(None);
    };
 for file in &files{remember_location(&state.data,location_kind,file.path(),false);}
    let files = files
        .into_iter()
        .map(|f| f.path().to_path_buf())
        .collect::<Vec<_>>();
    if files.len() > 100 {
        return Err("一次最多添加 100 份文件".into());
    }
    if (kind == "repairPaper" || kind == "notation" && target.is_some()) && files.len() != 1 {
        return Err("修复时请选择一份原文件".into());
    }
    let registry = state.score_imports.clone();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    if batch_id.len() > 64 || batch_id.is_empty() {
        return Err("导入批次无效".into());
    }
    {
        let mut map = registry.lock().map_err(|_| "导入队列正忙")?;
        if map.len() >= 8 || map.contains_key(&batch_id) {
            return Err("导入队列正忙或批次重复".into());
        }
        map.insert(batch_id.clone(), cancel.clone());
    }
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let total = files.len();
        let mut results = vec![];
        let mut append_to = target.clone();
        for (i, file) in files.into_iter().enumerate() {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) { break; }
            let name = file.file_name().unwrap_or_default().to_string_lossy().to_string();
            let _ = app.emit("library-score-import-progress", serde_json::json!({
                "batchId": batch_id, "current": i+1, "total": total,
                "name": name, "results": results
            }));
            let result = (|| -> Result<serde_json::Value, String> {
                let limit = if kind == "notation" { 4_000_000 } else { 48_000_000 };
                if std::fs::metadata(&file).map_err(|e|e.to_string())?.len() > limit {
                    return Err("文件超过当前谱面格式容量上限".into());
                }
                let bytes = std::fs::read(&file).map_err(|e|e.to_string())?;
                let hash=blake3::hash(&bytes).to_hex().to_string();
                let image = kind == "paper" && !bytes.starts_with(b"%PDF-");
                let actual = if image && append_to.is_some() { "append" } else { &kind };
                let value = engine.command(Command::AddLibraryScore {
                    path: path.clone().into(), content_id: content_id.clone(), name: name.clone(), bytes,
                    kind: actual.into(), target: if image { append_to.clone() } else { target.clone() }, page
                })?;
                if kind=="notation" {
                    engine.command(Command::LinkScoreSource {midi_path:path.clone().into(),content_id:content_id.clone(),version_id:value["id"].as_str().ok_or("谱面版本编号缺失")?.into(),path:file.clone(),default_bpm:0})
                        .map_err(|e|format!("谱面已保存，原文件关联失败：{e}"))?;
                }
                if ["paper","append"].contains(&kind.as_str()){link_imported_paper_source(&engine,&content_id,&value,file.clone(),&hash)?;}
                if kind=="repairPaper"{engine.command(Command::LinkPaperSource{content_id:content_id.clone(),book:target.clone().ok_or("待修复谱面未指定")?,asset:hash.clone(),path:Some(file.clone()),baseline:None})?;}
                if image && append_to.is_none() { append_to = value["active"].as_str().map(String::from); }
                Ok(value)
            })();
            results.push(match result {
                Ok(_) => serde_json::json!({"name":name,"ok":true}),
                Err(error) => serde_json::json!({"name":name,"ok":false,"error":error})
            });
        }
        let stopped = cancel.load(std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut map) = registry.lock() { map.remove(&batch_id); }
        Ok(Some(serde_json::json!({"results":results,"stopped":stopped,"total":total})))
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn cancel_library_score_import(
    state: tauri::State<'_, State>,
    batch_id: String,
) -> Result<(), String> {
    let map = state.score_imports.lock().map_err(|_| "导入队列正忙")?;
    if let Some(flag) = map.get(&batch_id) {flag.store(true, std::sync::atomic::Ordering::Relaxed);}
    Ok(())
}
#[tauri::command]
async fn pick_practice_backup(state:tauri::State<'_,State>)->Result<Option<serde_json::Value>,String>{
 let Some(file)=file_dialog(&state.data,"backup").set_title("预览练习资料备份").add_filter("练习备份", &["neopractice"]).pick_file().await else{return Ok(None)};let path=file.path().to_path_buf();remember_location(&state.data,"backup",file.path(),false);let engine=state.engine.clone();tauri::async_runtime::spawn_blocking(move||{if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>256_000_000{return Err("备份超过 256 MB".into());}engine.stage_practice_backup(std::fs::read(path).map_err(|e|e.to_string())?).map(Some)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn export_interface_settings(state:tauri::State<'_,State>,contents:String)->Result<Option<String>,String>{
 if contents.len()>1_000_000{return Err("界面设置备份超过 1 MB".into());}
 let value:serde_json::Value=serde_json::from_str(&contents).map_err(|_|"界面设置备份格式无效")?;
 if value["format"]!="neothesia-interface-settings"||value["version"]!=1{return Err("界面设置备份格式无效".into());}
 let Some(file)=file_dialog(&state.data,"exportBackup").set_title("导出界面设置").set_file_name("Neothesia-界面设置.neosettings").add_filter("界面设置备份", &["neosettings"]).save_file().await else{return Ok(None)};
 let path=file.path().to_path_buf();remember_location(&state.data,"exportBackup",&path,false);
 tauri::async_runtime::spawn_blocking(move||{std::fs::write(&path,contents).map_err(|e|e.to_string())?;Ok(Some(path.to_string_lossy().to_string()))}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn export_practice_backup(state:tauri::State<'_,State>,selection:neothesia_engine::practice_backup::Selection)->Result<Option<String>,String>{
 let engine=state.engine.clone();let v=tauri::async_runtime::spawn_blocking(move||engine.command(Command::ExportPracticeBackup{selection})).await.map_err(|e|e.to_string())??;let owned=v["path"].as_str().ok_or("备份导出路径无效")?.to_string();
 let Some(file)=file_dialog(&state.data,"exportBackup").set_title("保存练习资料备份").set_file_name("Neothesia-练习资料.neopractice").add_filter("练习备份", &["neopractice"]).save_file().await else{return Ok(None)};let dest=file.path().to_path_buf();remember_location(&state.data,"exportBackup",file.path(),false);tauri::async_runtime::spawn_blocking(move||{std::fs::copy(owned,&dest).map_err(|e|e.to_string())?;Ok(Some(dest.to_string_lossy().to_string()))}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn library_list(state: tauri::State<'_, State>) -> serde_json::Value {
    serde_json::json!({"songs": library::enriched(&state.songs.lock().unwrap(),&state.data)})
}
#[tauri::command]
fn engine_state(state: tauri::State<'_, State>) -> neothesia_engine::Snapshot {
    state.engine.snapshot()
}
#[tauri::command]
async fn engine_song(state: tauri::State<'_, State>) -> Result<serde_json::Value, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.command(Command::CurrentSong))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn midi_devices() -> serde_json::Value {
    neothesia_engine::devices()
}
#[tauri::command]
async fn engine_command(
    state: tauri::State<'_, State>,
    command: Command,
) -> Result<serde_json::Value, String> {
    if matches!(command,Command::LibraryMonitorStatus){return serde_json::to_value(state.monitor.status()?).map_err(|e|e.to_string());}
    if let Command::SetLibraryMonitor{enabled}=command {state.monitor.set_enabled(enabled)?;return serde_json::to_value(state.monitor.status()?).map_err(|e|e.to_string());}
    if matches!(command, Command::LibraryFolders) {
        return library::folder_status(&state.data);
    }
    if matches!(
        command,
        Command::RefreshLibrary | Command::RemoveLibraryFolder { .. }
    ) {
        if let Command::RemoveLibraryFolder{path}=command {library::remove_folder(&state.data,&path)?;}
        state.monitor.refresh()?;
        return Ok(serde_json::json!({"songs":state.monitor.rows()?,"scheduled":true}));
    }
    let paths = library::command_paths(&command);
    if !paths.is_empty() {
        let known = state.engine.command(Command::Collection)?;
        let indexed = state.engine.command(Command::LibraryWorkspace)?;
        let songs = state.songs.lock().map_err(|e| e.to_string())?;
        library::validate_paths(paths, &songs, &known, &indexed)?;
    }
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.command(command))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn link_score_source(state:tauri::State<'_,State>,midi_path:String,content_id:String,version_id:String)->Result<Option<serde_json::Value>,String>{
    let Some(file)=file_dialog(&state.data,"notation").set_title("关联此版本的原谱文件").add_filter("MusicXML 乐谱", &["xml","musicxml","mxl"]).pick_file().await else{return Ok(None);};
    let path=file.path().to_path_buf();remember_location(&state.data,"notation",file.path(),false);let engine=state.engine.clone();
    tauri::async_runtime::spawn_blocking(move ||engine.command(Command::LinkScoreSource{midi_path:midi_path.into(),content_id,version_id,path,default_bpm:0}).map(Some)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn import_midi(state: tauri::State<'_, State>) -> Result<Option<serde_json::Value>, String> {
    let Some(file) = file_dialog(&state.data,"music")
        .set_title("打开 MIDI 或乐谱")
        .add_filter(
            "MIDI / MusicXML",
            &["mid", "midi", "musicxml", "xml", "mxl"],
        )
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let path = file.path().to_path_buf();remember_location(&state.data,"music",file.path(),false);
    let title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        if ["xml", "musicxml", "mxl"].contains(&extension.as_str()) {
            let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
            if size > 4_000_000 {
                return Err("乐谱超过 4 MB".into());
            }
            engine.command(Command::ImportScoreOriginal {path,default_bpm:120})
        } else {
            engine.command(Command::Load { path, title })
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .map(Some)
}
#[tauri::command]
async fn export_midi(state: tauri::State<'_, State>) -> Result<Option<String>, String> {
    let engine = state.engine.clone();
    let data = tauri::async_runtime::spawn_blocking(move || engine.command(Command::ExportMidi))
        .await
        .map_err(|e| e.to_string())??;
    let bytes = std::fs::read(data["path"].as_str().ok_or("导出路径无效")?).map_err(|e|e.to_string())?;
    let name = format!("{}.mid", data["title"].as_str().unwrap_or("演奏"));
    let Some(file) = file_dialog(&state.data,"exportMidi")
        .set_title("导出 MIDI")
        .add_filter("MIDI", &["mid"])
        .set_file_name(name)
        .save_file()
        .await
    else {
        return Ok(None);
    };
    remember_location(&state.data,"exportMidi",file.path(),false);
    file.write(&bytes).await.map_err(|e| e.to_string())?;
    Ok(Some(file.path().to_string_lossy().into()))
}
#[tauri::command]
async fn export_paper_page(state:tauri::State<'_,State>,name: String, bytes: Vec<u8>) -> Result<Option<String>, String> {
    if bytes.len()<33 || bytes.len()>48_000_000 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || &bytes[12..16]!=b"IHDR" {return Err("批注图片格式或大小无效".into());}
    let width=u32::from_be_bytes(bytes[16..20].try_into().unwrap());let height=u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width==0 || height==0 || height>16000 || u64::from(width)*u64::from(height)>32_000_000 {return Err("批注图片尺寸过大或无效".into());}
    let name=name.chars().map(|c|if "<>:\"/\\|?*".contains(c)||c.is_control(){'_'}else{c}).collect::<String>();
    let Some(file)=file_dialog(&state.data,"exportPaper").set_title("保存本页批注图片").add_filter("PNG 图片", &["png"])
        .set_file_name(if name.ends_with(".png"){name}else{format!("{name}.png")}).save_file().await else{return Ok(None);};
    remember_location(&state.data,"exportPaper",file.path(),false);
    file.write(&bytes).await.map_err(|e|e.to_string())?;Ok(Some(file.path().to_string_lossy().into()))
}

#[cfg(target_os="windows")]
fn installed_musescore() -> Option<PathBuf> {
    let mut roots = Vec::new();
    for key in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = std::env::var_os(key) { roots.push(PathBuf::from(root)); }
    }
    if let Some(root) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(root).join("Programs"));
    }
    for root in roots {
        for name in ["MuseScore 4", "MuseScore Studio 4"] {
            let path = root.join(name).join("bin").join("MuseScore4.exe");
            if path.is_file() { return Some(path); }
        }
    }
    std::env::split_paths(&std::env::var_os("PATH")?).map(|dir| dir.join("MuseScore4.exe")).find(|path|path.is_file())
}
#[tauri::command]
fn begin_located_paper_upload(state:tauri::State<'_,State>,size:usize)->Result<String,String>{
    if size<8||size>50_100_000{return Err("可定位纸谱数据容量无效".into());}
    let seed=format!("{}:{size}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
    let id=blake3::hash(seed.as_bytes()).to_hex().to_string();
    *state.located_paper_transfer.lock().map_err(|_|"谱页传输正忙")?=Some(LocatedPaperTransfer{id:id.clone(),size,bytes:Vec::with_capacity(size)});
    Ok(id)
}
#[tauri::command]
fn write_located_paper_chunk(state:tauri::State<'_,State>,session:String,offset:usize,bytes:Vec<u8>)->Result<(),String>{
    let mut guard=state.located_paper_transfer.lock().map_err(|_|"谱页传输正忙")?;
    let transfer=guard.as_mut().filter(|t|t.id==session).ok_or("谱页传输已失效")?;
    if bytes.is_empty()||bytes.len()>1_048_576||offset!=transfer.bytes.len()||offset.checked_add(bytes.len()).is_none_or(|n|n>transfer.size){return Err("谱页分块次序或容量无效".into());}
    transfer.bytes.extend_from_slice(&bytes);Ok(())
}
#[tauri::command]
async fn finish_located_paper_upload(state:tauri::State<'_,State>,session:String)->Result<serde_json::Value,String>{
    let transfer={
        let mut guard=state.located_paper_transfer.lock().map_err(|_|"谱页传输正忙")?;
        if guard.as_ref().is_none_or(|t|t.id!=session){return Err("谱页传输已失效".into());}
        guard.take().unwrap()
    };
    if transfer.bytes.len()!=transfer.size{return Err("谱页传输不完整".into());}
    let (meta,pages)=neothesia_engine::score_attachments::decode_located_paper_upload(&transfer.bytes)?;
    let engine=state.engine.clone();
    tauri::async_runtime::spawn_blocking(move ||engine.command(Command::CreateLocatedScorePaper{
        content_id:meta.content_id,score_revision:meta.score_revision,
        source_fingerprint:meta.source_fingerprint,mapping_token:meta.mapping_token,
        name:meta.name,pages,anchors:meta.anchors,
    })).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn cancel_located_paper_upload(state:tauri::State<'_,State>,session:String){
    if let Ok(mut guard)=state.located_paper_transfer.lock(){if guard.as_ref().is_some_and(|t|t.id==session){guard.take();}}
}

async fn musescore_executable(data:&std::path::Path)->Result<PathBuf,String>{
    #[cfg(target_os="windows")]
    {
        let saved=data.join("musescore-program.json");
        if let Ok(bytes)=std::fs::read(&saved){
            if let Ok(path)=serde_json::from_slice::<PathBuf>(&bytes){
                if path.is_file() && path.file_name().and_then(|name|name.to_str()).is_some_and(|name|name.eq_ignore_ascii_case("MuseScore4.exe")){return Ok(path)}
            }
        }
        if let Some(path)=installed_musescore(){return Ok(path)}
        let Some(file)=rfd::AsyncFileDialog::new().set_title("选择 MuseScore Studio 程序")
            .add_filter("MuseScore 程序", &["exe"]).pick_file().await else {
            return Err("未找到 MuseScore Studio。请安装程序，或选择便携版 MuseScore4.exe".into());
        };
        let path=std::fs::canonicalize(file.path()).map_err(|e|format!("MuseScore 程序无法读取：{e}"))?;
        if path.file_name().and_then(|name|name.to_str()).is_none_or(|name|!name.eq_ignore_ascii_case("MuseScore4.exe")) {
            return Err("请选择 MuseScore Studio 的 MuseScore4.exe 程序".into());
        }
        std::fs::write(saved,serde_json::to_vec(&path).map_err(|e|e.to_string())?).map_err(|e|format!("MuseScore 程序位置无法保存：{e}"))?;
        Ok(path)
    }
    #[cfg(not(target_os="windows"))]
    {let _=data;Ok(PathBuf::from(if cfg!(target_os="macos"){"open"}else{"musescore"}))}
}
#[cfg(target_os="windows")]
fn musescore_path_arg(path:&std::path::Path)->String{
    let text=path.to_string_lossy();
    if let Some(unc)=text.strip_prefix(r"\\?\UNC\"){format!(r"\\{unc}")}
    else{text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()}
}

#[tauri::command]
async fn open_score_in_musescore(state:tauri::State<'_,State>,content_id:String,version_id:String)->Result<(),String>{
    let data=state.data.clone();
    // The frontend supplies identities only. The engine resolves and hashes the registered file.
    let path=tauri::async_runtime::spawn_blocking(move||neothesia_engine::score_sources::original_for_edit(&data,&content_id,&version_id)).await.map_err(|e|e.to_string())??;
    launch_score_in_musescore(&state.data,&path).await
}

async fn launch_score_in_musescore(data:&std::path::Path,path:&std::path::Path)->Result<(),String>{
    #[cfg(target_os="windows")]
    let executable=musescore_executable(data).await?;
    #[cfg(target_os="windows")]
    {std::process::Command::new(executable).arg(musescore_path_arg(&path)).spawn().map_err(|e|format!("MuseScore Studio 启动失败：{e}"))?;Ok(())}
    #[cfg(not(target_os="windows"))]
    {let _=data;let program=if cfg!(target_os="macos"){"open"}else{"musescore"};let mut command=std::process::Command::new(program);if cfg!(target_os="macos"){command.arg("-a").arg("MuseScore 4");}command.arg(path).spawn().map_err(|e|format!("MuseScore Studio 启动失败：{e}"))?;Ok(())}
}

#[cfg(target_os="windows")]
fn convert_musescore_pdf(executable:&std::path::Path,source:&std::path::Path,destination:&std::path::Path)->Result<Vec<u8>,String>{
    use std::os::windows::process::CommandExt;
    let mut child=std::process::Command::new(executable).arg("-o").arg(musescore_path_arg(destination)).arg(musescore_path_arg(source))
        .creation_flags(0x08000000).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e|format!("MuseScore PDF 导出无法启动：{e}"))?;
    let start=std::time::Instant::now();
    loop{
        if let Some(status)=child.try_wait().map_err(|e|format!("MuseScore PDF 导出状态无法读取：{e}"))?{
            if !status.success(){return Err(format!("MuseScore PDF 导出失败，退出码：{}",status.code().unwrap_or(-1)))}
            break;
        }
        if start.elapsed()>std::time::Duration::from_secs(60){let _=child.kill();let _=child.wait();return Err("MuseScore PDF 导出超过 60 秒，已停止".into())}
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let bytes=std::fs::read(destination).map_err(|e|format!("MuseScore 没有生成 PDF：{e}"))?;
    if bytes.len()<100||bytes.len()>48_000_000||!bytes.starts_with(b"%PDF-")||!bytes[bytes.len().saturating_sub(1024)..].windows(5).any(|part|part==b"%%EOF"){
        return Err("MuseScore 导出的 PDF 不完整或超过 48 MB，未加入谱面".into())
    }
    Ok(bytes)
}

#[cfg(target_os="windows")]
fn convert_musescore_midi_to_musicxml(executable:&std::path::Path,source:&std::path::Path,destination:&std::path::Path)->Result<Vec<u8>,String>{
    use std::os::windows::process::CommandExt;
    let mut child=std::process::Command::new(executable).arg("-o").arg(musescore_path_arg(destination)).arg(musescore_path_arg(source))
        .creation_flags(0x08000000).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e|format!("MuseScore MIDI 转谱无法启动：{e}"))?;
    let start=std::time::Instant::now();
    loop {
        if let Some(status)=child.try_wait().map_err(|e|format!("MuseScore MIDI 转谱状态无法读取：{e}"))? {
            if !status.success(){return Err(format!("MuseScore MIDI 转谱失败，退出码：{}",status.code().unwrap_or(-1)))}
            break;
        }
        if start.elapsed()>std::time::Duration::from_secs(90){let _=child.kill();let _=child.wait();return Err("MuseScore MIDI 转谱超过 90 秒，已停止".into())}
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let size=std::fs::metadata(destination).map_err(|e|format!("MuseScore 没有生成 MusicXML：{e}"))?.len();
    if !(100..=4_000_000).contains(&size){return Err("MuseScore 生成的 MusicXML 不完整或超过 4 MB，未加入谱面".into())}
    let bytes=std::fs::read(destination).map_err(|e|format!("MuseScore 生成的 MusicXML 无法读取：{e}"))?;
    let xml=std::str::from_utf8(&bytes).map_err(|_|"MuseScore 生成的 MusicXML 不是 UTF-8，未加入谱面")?;
    if !xml.contains("<score-partwise") && !xml.contains("<score-timewise") {return Err("MuseScore 生成的文件不是 MusicXML 乐谱".into())}
    Ok(bytes)
}

#[tauri::command]
async fn generate_musescore_score(state:tauri::State<'_,State>,content_id:String)->Result<serde_json::Value,String>{
    let current=state.engine.command(Command::CurrentSong)?;
    if current["contentId"].as_str()!=Some(&content_id){return Err("曲目已切换，请重新打开谱面管理".into())}
    let source=PathBuf::from(current["sourcePath"].as_str().ok_or("当前曲目没有 MIDI 文件")?);
    let ext=source.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext!="mid"&&ext!="midi" {return Err("当前曲目来自乐谱文件，请直接管理已有原谱".into())}
    let size=std::fs::metadata(&source).map_err(|e|format!("MIDI 文件无法读取：{e}"))?.len();
    if size==0||size>20_000_000{return Err("MIDI 文件为空或超过 20 MB".into())}
    if midi_file::MidiFile::new(&source).map_err(|e|format!("MIDI 文件无法读取：{e}"))?.content_id!=content_id {
        return Err("MIDI 文件内容已改变，请重新打开曲目".into())
    }
    let executable=musescore_executable(&state.data).await?;
    #[cfg(target_os="windows")]
    {
        let data=state.data.clone();let engine=state.engine.clone();let generated=state.generated_scores.clone();
        tauri::async_runtime::spawn_blocking(move||{
            let dir=data.join("musescore-midi-temp");std::fs::create_dir_all(&dir).map_err(|e|format!("临时目录无法创建：{e}"))?;
            let unique=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
            let output=dir.join(format!("score-{}-{unique}.musicxml",std::process::id()));
            let result=(||{
                let bytes=convert_musescore_midi_to_musicxml(&executable,&source,&output)?;
                let latest=engine.command(Command::CurrentSong)?;
                if latest["contentId"].as_str()!=Some(&content_id){return Err("曲目已切换，请重新生成谱面".into())}
                if midi_file::MidiFile::new(&source).map_err(|e|format!("MIDI 文件无法复查：{e}"))?.content_id!=content_id {
                    return Err("MIDI 文件内容已改变，请重新生成谱面".into())
                }
                let stem=source.file_stem().and_then(|s|s.to_str()).unwrap_or("当前曲目").chars().take(80).collect::<String>();
                let name=format!("{stem} · MuseScore 转谱初稿.musicxml");
                let hash=blake3::hash(&bytes).to_hex().to_string();
                let token=blake3::hash(format!("{content_id}\n{hash}\n{unique}\n{}",std::process::id()).as_bytes()).to_hex().to_string();
                let mut previews=generated.lock().map_err(|_|"转谱预览资料正忙")?;
                previews.clear();
                previews.insert(token.clone(),GeneratedScorePreview{content_id,source,name:name.clone(),hash});
                Ok(serde_json::json!({"name":name,"bytes":bytes,"previewToken":token}))
            })();
            let _=std::fs::remove_file(output);
            result
        }).await.map_err(|e|e.to_string())?
    }
    #[cfg(not(target_os="windows"))]
    {let _=(source,executable);Err("当前平台尚未接入 MuseScore MIDI 转谱".into())}
}

fn write_new_generated_score(path:&std::path::Path,bytes:&[u8])->Result<(),String>{
    if path.extension().and_then(|s|s.to_str()).is_none_or(|ext|!ext.eq_ignore_ascii_case("musicxml")) {
        return Err("请将转谱初稿保存为 .musicxml 文件".into());
    }
    // create_new is the final no-overwrite guard even when the save dialog's
    // target changes between selection and the write.
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path)
        .map_err(|e|if e.kind()==std::io::ErrorKind::AlreadyExists {
            "该位置已有文件。请选择新的文件名；软件不会覆盖原谱".into()
        } else {format!("无法新建转谱文件：{e}")})?;
    use std::io::Write;
    if let Err(error)=file.write_all(bytes).and_then(|_|file.sync_all()) {
        drop(file);
        let _=std::fs::remove_file(path);
        return Err(format!("转谱文件写入失败：{error}"));
    }
    drop(file);
    let saved=neothesia_engine::score_sources::read_original(path)
        .map_err(|e|format!("转谱文件已写入，但无法复查：{e}；位置：{}",path.display()))?;
    if saved!=bytes {
        return Err(format!("转谱文件已写入，但内容核对失败；请检查文件：{}",path.display()));
    }
    Ok(())
}

#[tauri::command]
async fn save_generated_score_for_edit(state:tauri::State<'_,State>,content_id:String,name:String,bytes:Vec<u8>,preview_token:String)->Result<Option<serde_json::Value>,String>{
    let preview=state.generated_scores.lock().map_err(|_|"转谱预览资料正忙")?
        .get(&preview_token).cloned().ok_or("转谱预览已失效。请重新从 MIDI 生成谱面")?;
    if preview.content_id!=content_id || preview.name!=name || blake3::hash(&bytes).to_hex().as_str()!=preview.hash {
        return Err("转谱预览内容已改变。请重新从 MIDI 生成谱面".into());
    }
    let current=state.engine.command(Command::CurrentSong)?;
    if current["contentId"].as_str()!=Some(&content_id)
        || current["sourcePath"].as_str()!=Some(preview.source.to_string_lossy().as_ref()) {
        return Err("曲目已切换。请重新打开这首曲目的谱面管理".into());
    }
    if midi_file::MidiFile::new(&preview.source).map_err(|e|format!("MIDI 文件无法复查：{e}"))?.content_id!=content_id {
        return Err("MIDI 文件内容已改变。请重新生成谱面".into());
    }
    let Some(file)=file_dialog(&state.data,"notation").set_title("保存转谱初稿以供 MuseScore 编辑")
        .add_filter("MusicXML 乐谱", &["musicxml"]).set_file_name(&name).save_file().await else{return Ok(None)};
    let path=file.path().to_path_buf();
    let data=state.data.clone();let engine=state.engine.clone();let previews=state.generated_scores.clone();let cid_for_open=content_id.clone();
    let mut result=tauri::async_runtime::spawn_blocking(move||->Result<serde_json::Value,String>{
        let current=engine.command(Command::CurrentSong)?;
        if current["contentId"].as_str()!=Some(&content_id)
            || current["sourcePath"].as_str()!=Some(preview.source.to_string_lossy().as_ref()) {
            return Err("曲目已切换。转谱文件没有写入".into());
        }
        if midi_file::MidiFile::new(&preview.source).map_err(|e|format!("MIDI 文件无法复查：{e}"))?.content_id!=content_id {
            return Err("MIDI 文件内容已改变。转谱文件没有写入".into());
        }
        if previews.lock().map_err(|_|"转谱预览资料正忙")?.get(&preview_token)
            .is_none_or(|row|row.hash!=preview.hash || row.content_id!=content_id) {
            return Err("转谱预览已失效。转谱文件没有写入".into());
        }
        let parent=path.parent().ok_or("保存位置无效")?.canonicalize()
            .map_err(|e|format!("保存文件夹无法读取：{e}"))?;
        if std::fs::canonicalize(&data).is_ok_and(|root|parent.starts_with(root)) {
            return Err("请选择个人乐谱文件夹。软件的内部资料目录不能用作 MuseScore 编辑位置".into());
        }
        write_new_generated_score(&path,&bytes)?;
        let version=engine.command(Command::SaveScoreVersion{bytes:bytes.clone(),name:name.clone(),content_id:content_id.clone()})
            .map_err(|e|format!("转谱文件已保存到 {}，但谱面版本未保存：{e}",path.display()))?;
        let version_id=version["versionId"].as_str().ok_or_else(||format!("转谱文件已保存到 {}，但版本编号缺失",path.display()))?.to_string();
        // The registrar re-reads both the immutable version and the selected
        // external file. It rejects changed bytes before making a source link.
        engine.command(Command::LinkScoreSource{midi_path:preview.source.clone(),content_id:content_id.clone(),version_id:version_id.clone(),path:path.clone(),default_bpm:0})
            .map_err(|e|format!("转谱文件和版本已保存，但原谱关联未完成：{e}。文件：{}；版本：{version_id}",path.display()))?;
        previews.lock().map_err(|_|"转谱预览资料正忙")?.remove(&preview_token);
        Ok(serde_json::json!({"versionId":version_id,"path":path,"linked":true}))
    }).await.map_err(|e|e.to_string())??;
    remember_location(&state.data,"notation",file.path(),false);
    let version_id=result["versionId"].as_str().ok_or("保存版本编号缺失")?.to_string();
    let launch=match neothesia_engine::score_sources::original_for_edit(&state.data,&cid_for_open,&version_id) {
        Ok(path)=>launch_score_in_musescore(&state.data,&path).await,
        Err(error)=>Err(error),
    };
    match launch {
        Ok(())=>result["opened"]=serde_json::json!(true),
        Err(error)=>{
            result["opened"]=serde_json::json!(false);
            result["warning"]=serde_json::json!(format!("转谱文件和版本已保存，原谱已关联，但 MuseScore 未能打开：{error}"));
        }
    }
    Ok(Some(result))
}

#[tauri::command]
async fn add_musescore_pdf(state:tauri::State<'_,State>,content_id:String,version_id:String)->Result<serde_json::Value,String>{
    let data=state.data.clone();let engine=state.engine.clone();
    let source=neothesia_engine::score_sources::original_for_edit(&data,&content_id,&version_id)?;
    let executable=musescore_executable(&data).await?;
    #[cfg(target_os="windows")]
    {tauri::async_runtime::spawn_blocking(move||{
        let dir=data.join("musescore-pdf-temp");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
        let unique=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let pdf=dir.join(format!("export-{}-{unique}.pdf",std::process::id()));
        let result=(||{
            let bytes=convert_musescore_pdf(&executable,&source,&pdf)?;
            neothesia_engine::score_sources::original_for_edit(&data,&content_id,&version_id)?;
            let base=source.file_stem().and_then(|s|s.to_str()).unwrap_or("乐谱");
            engine.command(Command::AddScoreAttachment{content_id,name:format!("{base} · MuseScore.pdf"),bytes,append:None})
        })();
        let _=std::fs::remove_file(pdf);
        result
    }).await.map_err(|e|e.to_string())?}
    #[cfg(not(target_os="windows"))]
    {let _=(source,executable,engine);Err("当前平台尚未接入 MuseScore PDF 导出".into())}
}
#[tauri::command]
async fn export_paper_book(state:tauri::State<'_,State>,name:String,bytes:Vec<u8>) -> Result<Option<String>,String> {
    if bytes.len()<100 || bytes.len()>64_000_000 || !bytes.starts_with(b"%PDF-") || !bytes[bytes.len().saturating_sub(1024)..].windows(5).any(|part|part==b"%%EOF") {
        return Err("整本批注 PDF 格式或大小无效".into());
    }
    let name=name.chars().map(|c|if "<>:\"/\\|?*".contains(c)||c.is_control(){'_'}else{c}).collect::<String>();
    let Some(file)=file_dialog(&state.data,"exportPaper").set_title("保存整本批注 PDF").add_filter("PDF 文档", &["pdf"])
        .set_file_name(if name.ends_with(".pdf"){name}else{format!("{name}.pdf")}).save_file().await else{return Ok(None);};
    remember_location(&state.data,"exportPaper",file.path(),false);
    file.write(&bytes).await.map_err(|e|e.to_string())?;
    Ok(Some(file.path().to_string_lossy().into()))
}
#[tauri::command]
async fn export_annotated_score(state: tauri::State<'_, State>, request: serde_json::Value) -> Result<Option<String>, String> {
    let command: Command=serde_json::from_value(request).map_err(|e|e.to_string())?;
    if !matches!(command, Command::ExportAnnotatedScore {download:true,..}) {return Err("导出请求无效".into());}
    let engine=state.engine.clone();
    let data=tauri::async_runtime::spawn_blocking(move || engine.command(command)).await.map_err(|e|e.to_string())??;
    let bytes: Vec<u8>=serde_json::from_value(data["bytes"].clone()).map_err(|e|e.to_string())?;
    let Some(file)=file_dialog(&state.data,"exportScore").set_title("保存个人标记谱").add_filter("MusicXML 乐谱", &["musicxml"])
        .set_file_name(data["name"].as_str().unwrap_or("个人标记.musicxml")).save_file().await else {return Ok(None);};
    remember_location(&state.data,"exportScore",file.path(),false);
    file.write(&bytes).await.map_err(|e|e.to_string())?;
    Ok(Some(file.path().to_string_lossy().into()))
}
#[tauri::command]
async fn pick_package(state:tauri::State<'_,State>)->Result<Option<serde_json::Value>,String> {
    let Some(file)=file_dialog(&state.data,"piece").set_title("选择曲目包").add_filter("Neothesia 曲目包", &["neopiece"]).pick_file().await else {return Ok(None);};
    let path=file.path().to_path_buf();remember_location(&state.data,"piece",file.path(),false);let engine=state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>256_000_000 {return Err("曲目包超过 256 MB".into());}
        let mut result=engine.stage_package(std::fs::read(&path).map_err(|e|e.to_string())?)?;
        result["fileName"]=serde_json::json!(path.file_name().unwrap_or_default().to_string_lossy());Ok(Some(result))
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn stage_package(state:tauri::State<'_,State>,bytes:Vec<u8>)->Result<serde_json::Value,String> {
    let engine=state.engine.clone();tauri::async_runtime::spawn_blocking(move ||engine.stage_package(bytes)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn export_package(state: tauri::State<'_, State>) -> Result<Option<String>, String> {
    let engine = state.engine.clone();
    let data = tauri::async_runtime::spawn_blocking(move || engine.command(Command::ExportPackageFile))
        .await
        .map_err(|e| e.to_string())??;
    let bytes = std::fs::read(data["path"].as_str().ok_or("导出路径无效")?).map_err(|e|e.to_string())?;
    let name = data["title"]
        .as_str()
        .unwrap_or("曲目")
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect::<String>();
    let Some(file) = file_dialog(&state.data,"exportPiece")
        .set_title("导出曲目包")
        .add_filter("Neothesia 曲目包", &["neopiece"])
        .set_file_name(format!("{name}.neopiece"))
        .save_file()
        .await
    else {
        return Ok(None);
    };
    remember_location(&state.data,"exportPiece",file.path(),false);
    file.write(&bytes).await.map_err(|e| e.to_string())?;
    Ok(Some(file.path().to_string_lossy().into()))
}
#[tauri::command]
async fn repair_midi(state:tauri::State<'_,State>,content_id:String,from:String)->Result<Option<serde_json::Value>,String>{
 let Some(file)=file_dialog(&state.data,"repair").set_title("选择同一曲目的新位置").add_filter("MIDI", &["mid","midi"]).pick_file().await else{return Ok(None);};
 let to=file.path().to_path_buf();remember_location(&state.data,"repair",file.path(),false);let engine=state.engine.clone();
 tauri::async_runtime::spawn_blocking(move||engine.command(Command::RelinkLibrary{items:vec![neothesia_engine::library_repairs::RelinkItem{content_id,from:PathBuf::from(from),to}]}).map(Some)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn relocate_midi(
    state: tauri::State<'_, State>,
    content_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let Some(file) = file_dialog(&state.data,"repair")
        .set_title("重新定位原 MIDI 文件")
        .add_filter("MIDI", &["mid", "midi"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let path = file.path().to_path_buf();remember_location(&state.data,"repair",file.path(),false);
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let parsed = midi_file::MidiFile::new(&path)?;
        if parsed.content_id != content_id {
            return Err("所选文件的内容与原曲目不同，请通过打开 MIDI 导入为新曲目".into());
        }
        let collection = engine.command(Command::Collection)?;
        let indexed = engine.command(Command::LibraryWorkspace)?;
        let original = collection["songs"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|r| r["contentId"].as_str() == Some(&content_id))
            })
            .and_then(|r| r["path"].as_str())
            .or_else(|| {
                indexed["entries"]
                    .as_object()
                    .and_then(|entries| {
                        entries
                            .values()
                            .find(|e| e["contentId"].as_str() == Some(&content_id))
                    })
                    .and_then(|e| e["path"].as_str())
            });
        if let Some(original) = original {
            library::recover_annotations(std::path::Path::new(original), &path, &content_id)?;
        }
        let title = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into();
        engine.command(Command::Load { path, title }).map(Some)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn import_folder(state: tauri::State<'_, State>) -> Result<Option<usize>, String> {
    let Some(folder) = file_dialog(&state.data,"library")
        .set_title("添加 MIDI 曲库文件夹")
        .pick_folder()
        .await
    else {
        return Ok(None);
    };
    remember_location(&state.data,"library",folder.path(),true);
    let data = state.data.clone();
    let path = folder.path().to_owned();
    let rows = tauri::async_runtime::spawn_blocking(move || library::add_folder(&data, path))
        .await
        .map_err(|e| e.to_string())??;
    let mut songs = state.songs.lock().map_err(|e| e.to_string())?;
    let mut added = 0;
    for row in rows {
        if !songs.iter().any(|s| s.path == row.path) {
            songs.push(row);
            added += 1;
        }
    }
    state.monitor.refresh()?;
    Ok(Some(added))
}
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let root = std::env::var_os("NEOTHESIA_LIBRARY_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"D:\Music\MIDI\PracticeLibrary"));
            let data = std::env::var_os("NEOTHESIA_DATA_ROOT").map(PathBuf::from).unwrap_or(app.path().app_data_dir()?);
            std::fs::create_dir_all(&data)?;
            let data_lock = std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(data.join("desktop.lock"))?;
            data_lock.try_lock().map_err(|_| "Neothesia 已经在运行，请从任务栏打开已有窗口")?;
            let bundled_font = app.path().resource_dir()?.join("default.sf2");
            let font = if bundled_font.exists() {
                bundled_font
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../default.sf2")
            };
            let songs = library::load(&root, &data)?;
            if !font.is_file() { return Err(format!("找不到钢琴音色文件：{}", font.display()).into()); }
            let (saved,warning)=saved_soundfont(&data);
            let selected_font=saved.unwrap_or_else(||font.clone());
            let status=soundfont_status(&selected_font,&font,warning);
            let engine = Engine::start(data.clone(), selected_font.clone(), false);
            let songs=std::sync::Arc::new(std::sync::Mutex::new(songs));
            let monitor=neothesia_engine::library_monitor::Monitor::start(root,data.clone(),songs.clone());
            app.manage(score_batch_export::Sessions::default());
            app.manage(practice_backup_bridge::Sessions::default());
            app.manage(State { engine, songs,monitor,data, font:selected_font, bundled_font:font, soundfont_status:std::sync::Arc::new(std::sync::Mutex::new(status)), soundfont_change:Default::default(), score_imports:Default::default(), generated_scores:Default::default(), located_paper_transfer:Default::default(), _data_lock: data_lock });
            if !headless_check() {
                if let Some(window) = app.get_webview_window("main") {
                    window.show()?;
                    startup_stage("window_shown");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            score_batch_export::begin_score_batch_export,score_batch_export::begin_workspace_export,score_batch_export::write_score_batch_export,score_batch_export::finish_score_batch_export,score_batch_export::cancel_score_batch_export,
            reveal_library_file,
            get_soundfont_status,
            select_soundfont,
            reset_soundfont,
            open_score_in_musescore,
            add_musescore_pdf,
            generate_musescore_score,
            save_generated_score_for_edit,
            library_list,
            engine_state,
            engine_song,
            midi_devices,
            engine_command,
            begin_located_paper_upload,write_located_paper_chunk,finish_located_paper_upload,cancel_located_paper_upload,
            import_midi,
            link_score_source,
            export_midi,
            export_annotated_score,
            export_paper_page,
            export_paper_book,
            export_package,
            pick_package,
            stage_package,
            relocate_midi,
            repair_midi,
            score_attachment_bytes,
            import_score_attachments,
            pick_paper_source,
            import_library_scores,
            practice_backup_bridge::begin_practice_backup_read,
            practice_backup_bridge::read_practice_backup_chunk,
            practice_backup_bridge::finish_practice_backup_read,
            practice_backup_bridge::begin_practice_backup_stage,
            practice_backup_bridge::write_practice_backup_chunk,
            practice_backup_bridge::finish_practice_backup_stage,
            practice_backup_bridge::cancel_practice_backup_transfer,
            pick_practice_backup,
            export_practice_backup,
            export_interface_settings,
            cancel_library_score_import,
            import_folder,
            native_startup_check
        ])
        .on_page_load(|window, payload| {
            if startup_check() && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                startup_stage(&format!("page_finished:{}",payload.url()));
                if let Err(error) = window.eval(r#"
                    (function check(attempt) {
                        const ready = document.querySelector('h1') && document.body.innerText.includes('本地引擎已连接');
                        if ((ready || attempt > 100) && !window.__neothesiaChecking) {
                          window.__neothesiaChecking=true;
                          const worker=new Worker('/verovio/worker.mjs',{type:'module'});
                          const finish=async(notationReady)=>{worker.terminate();let pdfReady=false;try{pdfReady=Boolean(await Promise.race([window.__neothesiaPdfCheck?.(),new Promise(resolve=>setTimeout(()=>resolve(false),15000))]));}catch{}window.__TAURI_INTERNALS__.invoke('native_startup_check',{ready:Boolean(ready),notationReady,pdfReady});};
                          const timeout=setTimeout(()=>finish(false),15000);
                          worker.onmessage=({data})=>{clearTimeout(timeout);finish(!data.error && data.pages?.length>0 && Boolean(data.midi));};
                          worker.onerror=()=>{clearTimeout(timeout);finish(false);};
                          const xml='<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure></part></score-partwise>';
                          worker.postMessage({bytes:Array.from(new TextEncoder().encode(xml)),compressed:false});
                        }
                        else setTimeout(() => check(attempt + 1), 100);
                    })(0);
                "#) { startup_stage(&format!("eval_failed:{error}")); }
            }
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                if let Some(state) = window.try_state::<State>() {
                    let _ = state.engine.command(Command::Pause);
                }
            }
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            let message = format!("Neothesia 启动失败：{error}\n请保留程序目录中的 default.sf2，并确认已安装 Microsoft Edge WebView2。");
            let log = std::env::temp_dir().join("neothesia-startup-error.txt");
            let _ = std::fs::write(&log, &message);
            rfd::MessageDialog::new().set_title("Neothesia 启动失败").set_description(format!("{message}\n诊断文件：{}", log.display())).set_level(rfd::MessageLevel::Error).show();
        });
}

#[cfg(all(test,target_os="windows"))]
mod musescore_tests {
    use super::*;
    #[test]
    fn missing_saved_soundfont_returns_to_bundled_font_with_warning(){
        let data=std::env::temp_dir().join(format!("neothesia-soundfont-setting-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&data).unwrap();
        assert_eq!(saved_soundfont(&data),(None,None));
        let missing=data.join("removed.sf2");
        save_soundfont(&data,&missing).unwrap();
        save_soundfont(&data,&missing).unwrap();
        let (selected,warning)=saved_soundfont(&data);
        assert!(selected.is_none());
        assert!(warning.unwrap().contains("已找不到"));
        std::fs::remove_dir_all(&data).unwrap();
    }
    #[test]
    fn installed_musescore_general_exposes_gm_instruments_and_audio(){
        let path=std::path::Path::new(r"D:\Music\SoundFonts\MuseScore General\MuseScore_General.sf2");
        if !path.is_file(){return}
        neothesia_engine::validate_soundfont(path).unwrap();
        let data=std::env::temp_dir().join(format!("neothesia-gm-font-{}",std::process::id()));
        std::fs::create_dir_all(&data).unwrap();
        save_soundfont(&data,path).unwrap();
        let (restored,warning)=saved_soundfont(&data);
        assert_eq!(restored.as_deref(),Some(path));
        assert!(warning.is_none());
        let engine=Engine::start(data.clone(),PathBuf::new(),true);
        let changed=engine.command(Command::SoundFont{path:path.to_path_buf()}).unwrap();
        let presets=changed["soundPresets"].as_array().unwrap();
        assert!(presets.iter().any(|preset|preset["program"]==40));
        neothesia_engine::probe_audio(path).unwrap();
        drop(engine);
        if data.parent()==Some(std::env::temp_dir().as_path()) && data.file_name().and_then(|name|name.to_str()).is_some_and(|name|name.starts_with("neothesia-gm-font-")){
            let _=std::fs::remove_dir_all(data);
        }
    }
    #[test]
    fn score_path_arg_removes_extended_prefix(){
        assert_eq!(musescore_path_arg(std::path::Path::new(r"\\?\C:\Music\score.mxl")),r"C:\Music\score.mxl");
        assert_eq!(musescore_path_arg(std::path::Path::new(r"\\?\UNC\server\share\score.mxl")),r"\\server\share\score.mxl");
    }
    #[test]
    fn installed_musescore_converts_real_musicxml_to_pdf(){
        let Some(program)=installed_musescore() else{return};
        let input=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../neothesia-core/src/score_performance_test.musicxml");
        let output=std::env::temp_dir().join(format!("neothesia-musescore-test-{}.pdf",std::process::id()));
        let bytes=convert_musescore_pdf(&program,&input,&output).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let _=std::fs::remove_file(output);
    }
    #[test]
    fn installed_musescore_converts_real_midi_to_musicxml(){
        let Some(program)=installed_musescore() else{return};
        let input=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../neothesia-engine/assets/five-finger.mid");
        let output=std::env::temp_dir().join(format!("neothesia-musescore-midi-{}.musicxml",std::process::id()));
        let bytes=convert_musescore_midi_to_musicxml(&program,&input,&output).unwrap();
        let xml=std::str::from_utf8(&bytes).unwrap();
        assert!(xml.contains("<score-partwise"));
        assert!(xml.contains("<measure"));
        assert!(xml.contains("<note"));
        let data=std::env::temp_dir().join(format!("neothesia-musescore-preview-{}",std::process::id()));
        let engine=Engine::start(data.clone(),PathBuf::new(),true);
        let song=engine.command(Command::Load{path:input.clone(),title:"MuseScore fixture".into()}).unwrap();
        let preview=engine.command(Command::PreviewScoreBytesRoute{
            bytes,name:"MuseScore 转谱初稿.musicxml".into(),
            content_id:song["contentId"].as_str().unwrap().into(),
        }).unwrap();
        assert_eq!(preview["contentId"],song["contentId"]);
        assert!(preview["fingerprint"].as_str().is_some());
        let external=std::env::temp_dir().join(format!("neothesia-musescore-linked-{}-{}.musicxml",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let bytes=std::fs::read(&output).unwrap();
        write_new_generated_score(&external,&bytes).unwrap();
        assert!(write_new_generated_score(&external,b"replacement").is_err());
        assert_eq!(std::fs::read(&external).unwrap(),bytes);
        let cid=song["contentId"].as_str().unwrap().to_string();
        let saved=engine.command(Command::SaveScoreVersion{bytes:bytes.clone(),name:"MuseScore 转谱初稿.musicxml".into(),content_id:cid.clone()}).unwrap();
        engine.command(Command::LinkScoreSource{midi_path:input.clone(),content_id:cid.clone(),version_id:saved["versionId"].as_str().unwrap().into(),path:external.clone(),default_bpm:0}).unwrap();
        let sources=engine.command(Command::ScoreSources{content_id:Some(cid.clone())}).unwrap();
        assert_eq!(sources["entries"][0]["status"],"ready");
        let mut changed=bytes.clone();changed.extend_from_slice(b"\n<!-- edited in MuseScore -->\n");
        std::fs::write(&external,changed).unwrap();
        let sources=engine.command(Command::ScoreSources{content_id:Some(cid)}).unwrap();
        assert_eq!(sources["entries"][0]["status"],"changed");
        let source_preview=engine.command(Command::PreviewScoreSource{id:sources["entries"][0]["id"].as_str().unwrap().into(),fingerprint:sources["entries"][0]["fingerprint"].as_str().unwrap().into()}).unwrap();
        assert_eq!(source_preview["performanceReady"],true);
        drop(engine);
        if data.parent()==Some(std::env::temp_dir().as_path()) && data.file_name().and_then(|name|name.to_str()).is_some_and(|name|name.starts_with("neothesia-musescore-preview-")) {
            let _=std::fs::remove_dir_all(data);
        }
        let _=std::fs::remove_file(output);
        let _=std::fs::remove_file(external);
    }
}
