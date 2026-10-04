use neothesia_engine::{Command, Engine, devices, library};
use std::{path::PathBuf, sync::{Arc,Mutex}};
use tiny_http::{Header, Method, Response, Server};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    let root = std::env::var_os("NEOTHESIA_LIBRARY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\Music\MIDI\PracticeLibrary"));
    let data = std::env::var_os("NEOTHESIA_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work/web-data"));
    let font = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2");
    if args.iter().any(|a| a == "--audio-probe") {
        neothesia_engine::probe_audio(&font)?;
        println!("内置钢琴音色已加载，真实音频输出流初始化成功（未播放音符）");
        return Ok(());
    }
    let rows = Arc::new(Mutex::new(library::load(&root, &data)?));
    let monitor = neothesia_engine::library_monitor::Monitor::start(root.clone(),data.clone(),rows.clone());
    let engine = Engine::start(data.clone(), font, args.iter().any(|a| a == "--silent"));
    let address = std::env::var("NEOTHESIA_SERVICE_ADDR").unwrap_or("127.0.0.1:32124".into());
    if !address.starts_with("127.0.0.1:") {
        return Err("服务只允许监听本机回环地址".into());
    }
    let server = Server::http(&address)?;
    println!("Neothesia music engine: http://{address}");
    for mut request in server.incoming_requests() {
        let origin = request
            .headers()
            .iter()
            .find(|h| h.field.equiv("Origin"))
            .map(|h| h.value.as_str());
        let allowed_origin =
            origin.filter(|o| ["http://127.0.0.1:5173", "http://localhost:5173"].contains(o));
        if origin.is_some() && allowed_origin.is_none() {
            let _ =
                request.respond(Response::from_string("拒绝非本地界面请求").with_status_code(403));
            continue;
        }
        let origin_header = allowed_origin.map(str::to_owned);
        let url = request.url().to_owned();
        if request.method()==&Method::Post && ["/api/practice-backup/stage","/api/practice-backup/export"].contains(&url.as_str()) {
          let result=(||->Result<(Vec<u8>,String),String>{if !request.headers().iter().any(|h|h.field.equiv("X-Neothesia-Client")&&h.value.as_str()=="web"){return Err("缺少本地客户端标识".into());}
           if url.ends_with("/stage"){let mut bytes=vec![];request.as_reader().take(256_000_001).read_to_end(&mut bytes).map_err(|e|e.to_string())?;let v=engine.stage_practice_backup(bytes)?;Ok((serde_json::to_vec(&v).unwrap(),"application/json".into()))}
           else{let mut bytes=vec![];request.as_reader().take(4097).read_to_end(&mut bytes).map_err(|e|e.to_string())?;if bytes.len()>4096{return Err("备份条件请求过大".into());}let selection=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;let v=engine.command(Command::ExportPracticeBackup{selection})?;let bytes=std::fs::read(v["path"].as_str().ok_or("备份导出路径无效")?).map_err(|e|e.to_string())?;Ok((bytes,"application/zip".into()))}
          })();let(status,bytes,mime)=match result{Ok((b,m))=>(200,b,m),Err(e)=>(400,serde_json::to_vec(&serde_json::json!({"error":e})).unwrap(),"application/json".into())};let mut response=Response::from_data(bytes).with_status_code(status).with_header(Header::from_bytes("Content-Type",mime).unwrap());if let Some(origin)=origin_header{response.add_header(Header::from_bytes("Access-Control-Allow-Origin",origin).unwrap());}let _=request.respond(response);continue;
        }
        if request.method()==&Method::Post && ["/api/package/stage","/api/package/export"].contains(&url.as_str()) {
            let result=(||->Result<(Vec<u8>,String),String>{
                if !request.headers().iter().any(|h|h.field.equiv("X-Neothesia-Client")&&h.value.as_str()=="web") {return Err("缺少本地客户端标识".into());}
                if url.ends_with("/stage") {
                    let mut bytes=Vec::new();request.as_reader().take(256_000_001).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
                    let v=engine.stage_package(bytes)?;Ok((serde_json::to_vec(&v).map_err(|e|e.to_string())?,"application/json".into()))
                } else {
                    let v=engine.command(Command::ExportPackageFile)?;
                    let bytes=std::fs::read(v["path"].as_str().ok_or("导出路径无效")?).map_err(|e|e.to_string())?;Ok((bytes,"application/zip".into()))
                }
            })();
            let(status,bytes,mime)=match result {Ok((b,m))=>(200,b,m),Err(e)=>(400,serde_json::to_vec(&serde_json::json!({"error":e})).unwrap(),"application/json".into())};
            let mut response=Response::from_data(bytes).with_status_code(status).with_header(Header::from_bytes("Content-Type",mime).unwrap());
            if let Some(origin)=origin_header {response.add_header(Header::from_bytes("Access-Control-Allow-Origin",origin).unwrap());}
            let _=request.respond(response);continue;
        }
        if request.method()==&Method::Post && ["/api/score-attachment/add","/api/library-score/add","/api/library-score/check","/api/score-attachment/read"].contains(&url.as_str()){
            let result=(||->Result<(Vec<u8>,String),String>{
              if !request.headers().iter().any(|h|h.field.equiv("X-Neothesia-Client")&&h.value.as_str()=="web"){return Err("缺少本地客户端标识".into())}
              if url.ends_with("/check") {
                let header=|field:&'static str|request.headers().iter().find(|h|h.field.equiv(field)).map(|h|h.value.as_str().to_owned());
                let path=decode_name(&header("X-Neothesia-Path").ok_or("未指定曲目位置")?)?;
                let content_id=header("X-Neothesia-Song").ok_or("未指定曲目")?;
                let kind=header("X-Neothesia-Kind").ok_or("未指定谱面类型")?;
                let target=header("X-Neothesia-Target");
                let mut bytes=Vec::new();request.as_reader().take(48_000_001).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
                let result=engine.command(Command::CheckLibraryScoreImport{path:path.into(),content_id,bytes,kind,target})?;
                return Ok((serde_json::to_vec(&result).unwrap(),"application/json".into()));
              }
              if url.ends_with("/add"){
                let cid=request.headers().iter().find(|h|h.field.equiv("X-Neothesia-Song")).map(|h|h.value.as_str().to_owned()).ok_or("未指定曲目")?;
                let encoded=request.headers().iter().find(|h|h.field.equiv("X-Neothesia-Name")).map(|h|h.value.as_str().to_owned()).ok_or("未指定谱面名称")?;
                let name=decode_name(&encoded)?;
                let append=request.headers().iter().find(|h|h.field.equiv("X-Neothesia-Append")).map(|h|h.value.as_str().to_owned());
                let mut bytes=Vec::new();request.as_reader().take(48_000_001).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
                let value=if url=="/api/library-score/add" {let header=|field:&'static str|request.headers().iter().find(|h|h.field.equiv(field)).map(|h|h.value.as_str().to_owned());let path=decode_name(&header("X-Neothesia-Path").ok_or("未指定曲目位置")?)?;let kind=header("X-Neothesia-Kind").ok_or("未指定谱面类型")?;let target=header("X-Neothesia-Target");let page=header("X-Neothesia-Page").unwrap_or_else(||"0".into()).parse().map_err(|_|"页码无效")?;engine.command(Command::AddLibraryScore{path:path.into(),content_id:cid,name,bytes,kind,target,page})?}else{engine.command(Command::AddScoreAttachment{content_id:cid,name,bytes,append})?};
                Ok((serde_json::to_vec(&value).unwrap(),"application/json".into()))
              }else{
                let mut b=String::new();request.as_reader().take(4097).read_to_string(&mut b).map_err(|e|e.to_string())?;if b.len()>4096{return Err("请求过大".into())}
                let v:serde_json::Value=serde_json::from_str(&b).map_err(|e|e.to_string())?;
                let cid=v["contentId"].as_str().ok_or("未指定曲目")?;let id=v["id"].as_str().ok_or("未指定谱面")?;let(bytes,kind)=if let Some(k)=v["previewKind"].as_str(){neothesia_engine::score_attachments::read_source_preview(&data,cid,id,k)?}else{neothesia_engine::score_attachments::read(&data,cid,id,v["page"].as_u64().unwrap_or(0)as usize)?};
                Ok((bytes,match kind.as_str(){"pdf"=>"application/pdf","jpeg"=>"image/jpeg","png"=>"image/png",_=>"image/webp"}.into()))
              }
            })();
            let(status,bytes,mime)=match result{Ok((b,m))=>(200,b,m),Err(e)=>(400,serde_json::to_vec(&serde_json::json!({"error":e})).unwrap(),"application/json".into())};
            let mut response=Response::from_data(bytes).with_status_code(status).with_header(Header::from_bytes("Content-Type",mime).unwrap());
            if let Some(origin)=origin_header{response.add_header(Header::from_bytes("Access-Control-Allow-Origin",origin).unwrap());}
            let _=request.respond(response);continue;
        }
        let result: Result<serde_json::Value, String> = (|| {
            if request.method() == &Method::Options {
                return Ok(serde_json::json!({}));
            }
            match (request.method(), url.as_str()) {
                (&Method::Get, "/api/state") => {
                    Ok(serde_json::to_value(engine.snapshot()).unwrap())
                }
                (&Method::Get, "/api/song") => engine.command(Command::CurrentSong),
                (&Method::Get, "/api/devices") => Ok(devices()),
                (&Method::Get, "/api/library") => Ok(
                    serde_json::json!({ "songs": library::enriched(&rows.lock().unwrap(),&data) }),
                ),
                (&Method::Post, "/api/command") => {
                    if !request
                        .headers()
                        .iter()
                        .any(|h| h.field.equiv("X-Neothesia-Client") && h.value.as_str() == "web")
                    {
                        return Err("缺少本地客户端标识".into());
                    }
                    let mut body = String::new();
                    request
                        .as_reader()
                        .take(16_000_001)
                        .read_to_string(&mut body)
                        .map_err(|e| e.to_string())?;
                    if body.len() > 16_000_000 {
                        return Err("请求过大".into());
                    }
                    let command: Command =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    if matches!(command,Command::LibraryMonitorStatus){return serde_json::to_value(monitor.status()?).map_err(|e|e.to_string());}
                    if let Command::SetLibraryMonitor{enabled}=command {monitor.set_enabled(enabled)?;return serde_json::to_value(monitor.status()?).map_err(|e|e.to_string());}
                    if matches!(command, Command::LibraryFolders) {
                        return library::folder_status(&data);
                    }
                    if matches!(
                        command,
                        Command::RefreshLibrary | Command::RemoveLibraryFolder { .. }
                    ) {
                        if let Command::RemoveLibraryFolder { path } = command {
                            library::remove_folder(&data, &path)?;
                        }
                        monitor.refresh()?;
                        return Ok(serde_json::json!({"songs":monitor.rows()?,"scheduled":true}));
                    }
                    let paths = library::command_paths(&command);
                    if !paths.is_empty() {
                        let known = engine.command(Command::Collection)?;
                        let indexed = engine.command(Command::LibraryWorkspace)?;
                        let registered = rows.lock().unwrap();
                        library::validate_paths(paths, &registered, &known, &indexed)?;
                    }
                    engine.command(command)
                }
                _ => Err("接口不存在".into()),
            }
        })();
        let (status, value) = match result {
            Ok(v) => (200, v),
            Err(e) => (400, serde_json::json!({"error": e})),
        };
        let mut response = Response::from_string(value.to_string())
            .with_status_code(status)
            .with_header(
                Header::from_bytes("Content-Type", "application/json; charset=utf-8").unwrap(),
            )
            .with_header(
                Header::from_bytes(
                    "Access-Control-Allow-Headers",
                    "Content-Type, X-Neothesia-Path, X-Neothesia-Kind, X-Neothesia-Target, X-Neothesia-Page, X-Neothesia-Client, X-Neothesia-Song, X-Neothesia-Name, X-Neothesia-Append",
                )
                .unwrap(),
            )
            .with_header(
                Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap(),
            );
        if let Some(origin) = origin_header {
            response.add_header(Header::from_bytes("Access-Control-Allow-Origin", origin).unwrap());
        }
        let _ = request.respond(response);
    }
    Ok(())
}
use std::io::Read;

fn decode_name(value:&str)->Result<String,String>{let mut result=Vec::new();let b=value.as_bytes();let mut i=0;while i<b.len(){if b[i]==b'%'{if i+2>=b.len(){return Err("名称编码无效".into())}let s=std::str::from_utf8(&b[i+1..i+3]).map_err(|_|"名称编码无效")?;result.push(u8::from_str_radix(s,16).map_err(|_|"名称编码无效")?);i+=3;}else{result.push(b[i]);i+=1;}}String::from_utf8(result).map_err(|_|"名称编码无效".into())}
