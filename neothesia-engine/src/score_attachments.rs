pub mod source_monitor;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
const MAX_BYTES: usize = 48_000_000;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageAsset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub size: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct View {
    pub page: usize,
    pub zoom: u16,
    pub fit: bool,
    pub rotation: u16,
}
impl Default for View {
    fn default() -> Self {
        Self {
            page: 1,
            zoom: 100,
            fit: true,
            rotation: 0,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub format: String,
    pub pages: Vec<PageAsset>,
    pub view: View,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<Mapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub sources: std::collections::BTreeMap<String, PaperSource>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PaperSource { pub path: PathBuf, #[serde(default)] pub ignored: Option<String> }
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TransferAnnotation { pub id:String, pub asset_id:String, pub page:usize, pub x:f32, pub y:f32 }
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PaperTransfer {
    pub source:String,pub target:String,pub baseline:String,pub annotations:Vec<TransferAnnotation>,
    pub anchors:Vec<PageAnchor>,#[serde(default)]pub replace_anchors:bool,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all="camelCase")]
pub struct InkStroke {pub points:Vec<[f32;2]>,pub thickness:f32}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub id: String,
    pub asset_id: String,
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: String,
    #[serde(default,skip_serializing_if="Option::is_none")] pub width:Option<f32>,
    #[serde(default,skip_serializing_if="Option::is_none")] pub height:Option<f32>,
    #[serde(default,skip_serializing_if="Option::is_none")] pub ink:Option<InkStroke>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationDraft {
    pub asset_id: String,
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: String,
    #[serde(default,skip_serializing_if="Option::is_none")] pub width:Option<f32>,
    #[serde(default,skip_serializing_if="Option::is_none")] pub height:Option<f32>,
    #[serde(default,skip_serializing_if="Option::is_none")] pub ink:Option<InkStroke>,
}
pub fn validate_annotations(
    notes: &[Annotation],
    format: &str,
    assets: &[String],
) -> Result<(), String> {
    if notes.len() > 2000 {
        return Err("一份谱面最多 2000 条批注".into());
    }
    if notes.iter().filter_map(|n|n.ink.as_ref()).map(|i|i.points.len()).sum::<usize>()>200_000{return Err("一份谱面笔迹超过 20 万点，请分到其他谱面版本".into());}
    let mut ids = std::collections::BTreeSet::new();
    for n in notes {
        if n.id.is_empty()
            || n.id.len() > 80
            || !ids.insert(&n.id)
            || !assets.contains(&n.asset_id)
            || n.page == 0
            || n.page > 5000
            || (format == "images" && n.page != 1)
            || !n.x.is_finite()
            || !n.y.is_finite()
            || !(0.0..=1.0).contains(&n.x)
            || !(0.0..=1.0).contains(&n.y)
            || n.text.trim().is_empty()&&n.width.is_none()
            || match (n.width,n.height){(None,None)=>false,(Some(w),Some(h))=>!w.is_finite()||!h.is_finite()||w<0.002||h<0.002||n.x+w>1.000001||n.y+h>1.000001,_=>true}
            || n.ink.as_ref().is_some_and(|i|n.width.is_none()||n.height.is_none()||i.points.len()<2||i.points.len()>4096||!i.thickness.is_finite()||!(0.0005..=0.02).contains(&i.thickness)||i.points.iter().flatten().any(|p|!p.is_finite()||!(0.0..=1.0).contains(p)))
            || n.text.chars().count() > 400
            || n.text
                .chars()
                .any(|c| c.is_control() && !['\n', '\t'].contains(&c))
            || !["yellow", "blue", "green", "red"].contains(&n.color.as_str())
        {
            return Err("批注文字、位置、谱页身份或颜色无效".into());
        }
    }
    Ok(())
}
pub fn edit_annotation(
    data: &Path,
    cid: &str,
    book: &str,
    id: Option<&str>,
    draft: Option<AnnotationDraft>,
    expected: Option<Annotation>,
) -> Result<Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    let mut s = load(data, cid)?;
    let a = s
        .attachments
        .iter_mut()
        .find(|a| a.id == book)
        .ok_or("谱面版本不存在")?;
    let at = if let Some(id) = id {
        let at = a
            .annotations
            .iter()
            .position(|n| n.id == id)
            .ok_or("批注已被移除，请重新打开")?;
        if expected.as_ref() != Some(&a.annotations[at]) {
            return Err("批注已在其他窗口改变，请重新打开后修改".into());
        }
        Some(at)
    } else {
        if expected.is_some() {
            return Err("新批注条件无效".into());
        }
        None
    };
    if let Some(d) = draft {
        let note = Annotation {
            id: id.map(str::to_string).unwrap_or_else(|| {
                blake3::hash(
                    format!(
                        "{}:{}:{:?}",
                        cid,
                        a.annotations.len(),
                        std::time::SystemTime::now()
                    )
                    .as_bytes(),
                )
                .to_hex()
                .to_string()
            }),
            asset_id: d.asset_id,
            page: d.page,
            x: d.x,
            y: d.y,
            text: d.text.trim().to_string(),
            color: d.color,
            width:d.width,height:d.height,ink:d.ink,
        };
        if let Some(at) = at {
            a.annotations[at] = note;
        } else {
            a.annotations.push(note);
        }
        validate_annotations(
            &a.annotations,
            &a.format,
            &a.pages.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        )?;
    } else if let Some(at) = at {
        a.annotations.remove(at);
    } else {
        return Err("未指定批注".into());
    }
    save(data, &s)?;
    describe(data, &s)
}
/// Replace only this page's freehand marks; text and highlights remain independent.
pub fn edit_ink(data:&Path,cid:&str,book:&str,asset:&str,page:usize,expected:Vec<Annotation>,strokes:Vec<Annotation>)->Result<Value,String>{
 let lock=crate::library_workspace::lock_for(data)?;let _guard=lock.lock().map_err(|_|"谱页目录正忙")?;
 let mut store=load(data,cid)?;let a=store.attachments.iter_mut().find(|a|a.id==book).ok_or("谱面版本不存在")?;
 if !a.pages.iter().any(|p|p.id==asset)||page==0||page>5000||a.format=="images"&&page!=1{return Err("谱页身份或页码无效".into());}
 let same=|n:&Annotation|n.asset_id==asset&&n.page==page&&n.ink.is_some();
 let current=a.annotations.iter().filter(|n|same(n)).cloned().collect::<Vec<_>>();
 if current!=expected{return Err("本页笔迹已在其他窗口改变，请暂存草稿后重新核对".into());}
 if strokes.iter().any(|n|!same(n)){return Err("笔迹只能保存到当前页，未保存".into());}
 let mut notes=a.annotations.iter().filter(|n|!same(n)).cloned().collect::<Vec<_>>();notes.extend(strokes);
 validate_annotations(&notes,&a.format,&a.pages.iter().map(|p|p.id.clone()).collect::<Vec<_>>())?;
 a.annotations=notes;save(data,&store)?;describe(data,&store)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageAnchor {
    pub measure: usize,
    pub page: usize,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub region: Option<PageRegion>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all="camelCase")]
pub struct PageRegion { pub x:f32, pub y:f32, pub width:f32, pub height:f32 }
impl PageRegion {
    fn validate(&self)->Result<(),String>{
        if ![self.x,self.y,self.width,self.height].iter().all(|v|v.is_finite()) || self.x<0. || self.y<0. || self.width<0.002 || self.height<0.002 || self.x+self.width>1.000001 || self.y+self.height>1.000001 {
            return Err("谱页小节位置需在页内，区域不能为空".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub grid: String,
    pub enabled: bool,
    pub anchors: Vec<PageAnchor>,
}
impl Mapping {
    pub fn validate(&self, format: &str, image_pages: usize) -> Result<(), String> {
        if self.grid.is_empty()
            || self.grid.len() > 160
            || self.anchors.is_empty()
            || self.anchors.len() > 5000
        {
            return Err("谱页小节对应为空或超过容量".into());
        }
        let mut last = 0;
        for a in &self.anchors {
            if let Some(r)=&a.region {r.validate()?;}
            if a.measure <= last
                || a.measure > 1_000_000
                || a.page == 0
                || a.page > 5000
                || (format == "images" && a.page > image_pages)
            {
                return Err("小节需按演奏顺序排列、不能重复；谱页需在有效范围内".into());
            }
            last = a.measure;
        }
        Ok(())
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Store {
    content_id: String,
    active: Option<String>,
    attachments: Vec<Attachment>,
}
fn folder(data: &Path, cid: &str) -> Result<PathBuf, String> {
    if cid.is_empty() || cid.len() > 160 {
        return Err("曲目身份无效".into());
    }
    Ok(data
        .join("score-attachments")
        .join(blake3::hash(cid.as_bytes()).to_hex().as_str()))
}
fn load(data: &Path, cid: &str) -> Result<Store, String> {
    let p = folder(data, cid)?.join("manifest.json");
    let s = match std::fs::read(&p) {
        Ok(b) => {
            serde_json::from_slice::<Store>(&b).map_err(|e| format!("谱页目录读取失败：{e}"))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Store {
            content_id: cid.into(),
            ..Default::default()
        },
        Err(e) => return Err(e.to_string()),
    };
    if s.content_id != cid {
        return Err("谱页曲目身份不符".into());
    }
    Ok(s)
}
fn save(data: &Path, s: &Store) -> Result<(), String> {
    let dir = folder(data, &s.content_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("manifest.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(s).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, dir.join("manifest.json")).map_err(|e| e.to_string())
}
fn asset_path(data: &Path, cid: &str, a: &PageAsset) -> Result<PathBuf, String> {
    if a.id.len() != 64
        || !a.id.bytes().all(|c| c.is_ascii_hexdigit())
        || !["pdf", "png", "jpeg", "webp"].contains(&a.kind.as_str())
    {
        return Err("谱页索引无效".into());
    }
    Ok(folder(data, cid)?.join(format!("{}.{}", a.id, a.kind)))
}
fn title(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 120 {
        return Err("谱面名称需为 1 至 120 字".into());
    }
    Ok(n.into())
}
fn kind(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("单份谱面需小于 48 MB".into());
    }
    if bytes.starts_with(b"%PDF-") {
        Ok("pdf")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("png")
    } else if bytes.starts_with(&[255, 216, 255]) {
        Ok("jpeg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok("webp")
    } else {
        Err("只支持 PDF、PNG、JPEG 和 WebP；文件内容不是有效的支持格式".into())
    }
}
fn describe(data: &Path, s: &Store) -> Result<Value, String> {
    let mut v = serde_json::to_value(s).map_err(|e| e.to_string())?;
    for (a, item) in s
        .attachments
        .iter()
        .zip(v["attachments"].as_array_mut().unwrap())
    {
        for (p, out) in a.pages.iter().zip(item["pages"].as_array_mut().unwrap()) {
            out["available"] = json!(asset_path(data, &s.content_id, p)?.is_file());
        }
    }
    Ok(v)
}
pub fn list(data: &Path, cid: &str) -> Result<Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    describe(data, &load(data, cid)?)
}
fn source_bytes(path:&Path)->Result<Vec<u8>,String>{
    if !path.is_absolute(){return Err("请选择原件的完整文件位置".into());}
    let meta=std::fs::metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file() || meta.len()>MAX_BYTES as u64 {return Err("原件不是文件或超过 48 MB".into());}
    use std::io::Read;
    let mut bytes=Vec::new();std::fs::File::open(path).map_err(|e|e.to_string())?.take(MAX_BYTES as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    kind(&bytes)?;Ok(bytes)
}
fn attachment_baseline(a:&Attachment)->String {blake3::hash(&serde_json::to_vec(a).unwrap()).to_hex().to_string()}
pub fn link_source(data:&Path,cid:&str,book:&str,asset:&str,path:Option<PathBuf>,baseline:Option<&str>)->Result<Value,String>{
    let lock=crate::library_workspace::lock_for(data)?;let _g=lock.lock().map_err(|_|"谱页目录正忙")?;
    let mut store=load(data,cid)?;
    let a=store.attachments.iter_mut().find(|a|a.id==book).ok_or("谱面版本不存在")?;
    if baseline.is_some_and(|b|b!=attachment_baseline(a)){return Err("谱面或原件关联已改变，请刷新列表".into());}
    if !a.pages.iter().any(|p|p.id==asset){return Err("谱页已改变，请重新打开".into());}
    if let Some(path)=path {
        let bytes=source_bytes(&path)?;
        if blake3::hash(&bytes).to_hex().as_str()!=asset {return Err("所选文件与当前谱页内容不同。请添加为新版本，或选择内容相同的原件".into());}
        a.sources.insert(asset.into(),PaperSource{path,ignored:None});
    }else{a.sources.remove(asset);}
    save(data,&store)?;describe(data,&store)
}
pub fn inspect_sources(data:&Path,cid:&str,book:&str)->Result<Value,String>{
    let lock=crate::library_workspace::lock_for(data)?;let _g=lock.lock().map_err(|_|"谱页目录正忙")?;
    let store=load(data,cid)?;let a=store.attachments.iter().find(|a|a.id==book).ok_or("谱面版本不存在")?;
    let rows:Vec<_>=a.pages.iter().enumerate().map(|(i,p)|{
        let mut row=json!({"assetId":p.id,"page":i+1,"name":p.name,"status":"unlinked"});
        if let Some(source)=a.sources.get(&p.id){
            row["path"]=json!(source.path);
            match source_bytes(&source.path){
                Ok(bytes)=>{let hash=blake3::hash(&bytes).to_hex().to_string();row["fingerprint"]=json!(hash);
                    row["status"]=json!(if hash==p.id{"same"}else if source.ignored.as_ref()==Some(&hash){"ignored"}else{"changed"});}
                Err(e)=>{row["status"]=json!(if !source.path.exists(){"missing"}else{"unavailable"});row["error"]=json!(e);}
            }
        }row
    }).collect();
    Ok(json!({"rows":rows,"baseline":attachment_baseline(a)}))
}
pub fn preview_source(data:&Path,cid:&str,book:&str,asset:&str,fingerprint:&str)->Result<Value,String>{
    let lock=crate::library_workspace::lock_for(data)?;let _g=lock.lock().map_err(|_|"谱页目录正忙")?;
    let store=load(data,cid)?;let a=store.attachments.iter().find(|a|a.id==book).ok_or("谱面版本不存在")?;
    let source=a.sources.get(asset).ok_or("原件关联已改变，请重新检查")?;
    let bytes=source_bytes(&source.path)?;let hash=blake3::hash(&bytes).to_hex().to_string();
    if hash!=fingerprint || hash==asset {return Err("原件已变化，请重新检查后预览".into());}
    let k=kind(&bytes)?.to_string();
    if (a.format=="pdf")!=(k=="pdf"){return Err("原件格式已改变，请单独添加为新谱面".into());}
    let page=PageAsset{id:hash.clone(),name:source.path.file_name().unwrap_or_default().to_string_lossy().into(),kind:k,size:bytes.len()};
    let dest=asset_path(data,cid,&page)?;std::fs::write(dest,&bytes).map_err(|e|e.to_string())?;
    Ok(json!({"baseline":attachment_baseline(a),"fingerprint":hash,"asset":page,"name":a.name,"format":a.format}))
}
pub fn read_source_preview(data:&Path,cid:&str,hash:&str,k:&str)->Result<(Vec<u8>,String),String>{
    let page=PageAsset{id:hash.into(),name:String::new(),kind:k.into(),size:0};
    let bytes=source_bytes(&asset_path(data,cid,&page)?)?;
    if blake3::hash(&bytes).to_hex().as_str()!=hash || kind(&bytes)?!=k{return Err("预览缓存已变化，请重新检查".into());}
    Ok((bytes,k.into()))
}
pub fn apply_source(data:&Path,cid:&str,book:&str,asset:&str,fingerprint:&str,baseline:&str,action:&str)->Result<Value,String>{
    let lock=crate::library_workspace::lock_for(data)?;let _g=lock.lock().map_err(|_|"谱页目录正忙")?;
    let mut store=load(data,cid)?;let at=store.attachments.iter().position(|a|a.id==book).ok_or("谱面版本不存在")?;
    let old=&store.attachments[at];
    if attachment_baseline(old)!=baseline {return Err("谱面、批注、对应或原件位置已改变，请重新检查".into());}
    let page_at=old.pages.iter().position(|p|p.id==asset).ok_or("谱页已改变")?;
    let source=old.sources.get(asset).ok_or("原件关联已移除")?.clone();
    let bytes=source_bytes(&source.path)?;let hash=blake3::hash(&bytes).to_hex().to_string();
    if hash!=fingerprint || hash==asset {return Err("原件已再次变化，请重新检查".into());}
    if action=="ignore" {store.attachments[at].sources.get_mut(asset).unwrap().ignored=Some(hash);}
    else if action=="version" {
        let k=kind(&bytes)?.to_string();
        if (old.format=="pdf")!=(k=="pdf"){return Err("原件格式已改变，请单独添加为新谱面".into());}
        if store.attachments.len()>=40 {return Err("一首曲目最多 40 份谱面附件".into());}
        let total:usize=store.attachments.iter().flat_map(|a|&a.pages).map(|p|p.size).sum();
        let extra:usize=old.pages.iter().enumerate().map(|(i,p)|if i==page_at {bytes.len()}else{p.size}).sum();
        if total+extra>240_000_000{return Err("谱面附件总量超过 240 MB".into());}
        if old.pages.iter().enumerate().any(|(i,p)|i!=page_at&&p.id==hash){return Err("更新后的图片与同一谱面另一页重复，请单独整理".into());}
        let mut new=old.clone();let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        new.id=format!("paper-update-{stamp}");new.name=format!("{} · 更新",old.name.chars().take(114).collect::<String>());
        new.mapping=None;new.annotations.clear();new.view=View::default();
        new.pages[page_at]=PageAsset{id:hash.clone(),name:source.path.file_name().unwrap_or_default().to_string_lossy().into(),kind:k,size:bytes.len()};
        new.sources.remove(asset);new.sources.insert(hash.clone(),PaperSource{path:source.path,ignored:None});
        std::fs::write(asset_path(data,cid,&new.pages[page_at])?,bytes).map_err(|e|e.to_string())?;
        store.active=Some(new.id.clone());store.attachments.push(new);
    }else{return Err("更新操作无效".into());}
    save(data,&store)?;describe(data,&store)
}
pub fn set_mapping(
    data: &Path,
    cid: &str,
    id: &str,
    mapping: Option<Mapping>,
) -> Result<Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    let mut s = load(data, cid)?;
    let a = s
        .attachments
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or("谱面版本不存在")?;
    if let Some(m) = &mapping {
        m.validate(&a.format, a.pages.len())?;
    }
    a.mapping = mapping;
    save(data, &s)?;
    describe(data, &s)
}
fn transfer_id(source:&str,target:&str,id:&str)->String {blake3::hash(format!("paper-transfer:{source}:{target}:{id}").as_bytes()).to_hex().to_string()}
fn transfer_baseline(source:&Attachment,target:&Attachment)->String {blake3::hash(format!("{}:{}",attachment_baseline(source),attachment_baseline(target)).as_bytes()).to_hex().to_string()}
pub fn preview_transfer(data:&Path,cid:&str,source:&str,target:&str)->Result<Value,String>{
    if source==target{return Err("请选择不同的来源谱面".into());}
    let lock=crate::library_workspace::lock_for(data)?;let _g=lock.lock().map_err(|_|"谱页目录正忙")?;
    let s=load(data,cid)?;let a=s.attachments.iter().find(|a|a.id==source).ok_or("来源谱面不存在")?;
    let b=s.attachments.iter().find(|a|a.id==target).ok_or("目标谱面不存在")?;
    let described=describe(data,&s)?;let books=described["attachments"].as_array().unwrap();
    let imported:Vec<_>=a.annotations.iter().filter(|n|b.annotations.iter().any(|t|t.id==transfer_id(source,target,&n.id))).map(|n|&n.id).collect();
    Ok(json!({"source":books.iter().find(|v|v["id"]==source),"target":books.iter().find(|v|v["id"]==target),"baseline":transfer_baseline(a,b),"imported":imported}))
}
pub fn apply_transfer(data:&Path,cid:&str,request:PaperTransfer)->Result<Value,String>{
    if request.source==request.target || request.annotations.len()>2000 || request.anchors.len()>5000 || (request.annotations.is_empty()&&request.anchors.is_empty()){return Err("请选择来源谱面和要迁移的批注或小节对应".into());}
    let lock=crate::library_workspace::lock_for(data)?;let _g=lock.lock().map_err(|_|"谱页目录正忙")?;
    let mut s=load(data,cid)?;let source=s.attachments.iter().find(|a|a.id==request.source).ok_or("来源谱面不存在")?.clone();
    let target=s.attachments.iter_mut().find(|a|a.id==request.target).ok_or("目标谱面不存在")?;
    if transfer_baseline(&source,target)!=request.baseline{return Err("来源或目标谱面资料已改变，请重新审阅迁移".into());}
    let mut seen=std::collections::BTreeSet::new();let mut added=0;
    for edit in &request.annotations {
        if !seen.insert(&edit.id){return Err("迁移批注重复，未保存".into());}
        let note=source.annotations.iter().find(|n|n.id==edit.id).ok_or("来源批注已移除")?;
        let copy=Annotation{id:transfer_id(&request.source,&request.target,&edit.id),asset_id:edit.asset_id.clone(),page:edit.page,x:edit.x,y:edit.y,text:note.text.clone(),color:note.color.clone(),width:note.width,height:note.height,ink:note.ink.clone()};
        if let Some(old)=target.annotations.iter().find(|n|n.id==copy.id){if old!=&copy{return Err("这条批注已迁移且后来修改，请在目标谱面继续编辑，或取消这条迁移".into());}}
        else{target.annotations.push(copy);added+=1;}
    }
    validate_annotations(&target.annotations,&target.format,&target.pages.iter().map(|p|p.id.clone()).collect::<Vec<_>>())?;
    if !request.anchors.is_empty(){
        let mapping=source.mapping.as_ref().ok_or("来源谱面没有小节对应")?;
        let mut result=target.mapping.clone().unwrap_or(Mapping{grid:mapping.grid.clone(),enabled:false,anchors:vec![]});
        if result.grid!=mapping.grid{return Err("两份谱面的小节网格不同，请在目标谱面重新设置对应".into());}
        let mut seen=std::collections::BTreeSet::new();
        for anchor in &request.anchors{
            if !seen.insert(anchor.measure)||!mapping.anchors.iter().any(|a|a.measure==anchor.measure){return Err("来源小节对应无效或重复".into());}
            if let Some(old)=result.anchors.iter_mut().find(|a|a.measure==anchor.measure){
                if (old.page!=anchor.page||old.region!=anchor.region)&&!request.replace_anchors{return Err("目标已有同小节对应，请取消该项或明确勾选替换".into());}old.page=anchor.page;old.region=anchor.region.clone();
            }else{result.anchors.push(anchor.clone());}
        }
        result.anchors.sort_by_key(|a|a.measure);result.enabled=false;result.validate(&target.format,target.pages.len())?;target.mapping=Some(result);
    }
    save(data,&s)?;Ok(json!({"added":added,"anchors":request.anchors.len(),"saved":true}))
}
pub fn add(
    data: &Path,
    cid: &str,
    name: &str,
    bytes: Vec<u8>,
    append: Option<&str>,
) -> Result<Value, String> {
    let k = kind(&bytes)?.to_owned();
    let name = title(name)?;
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    let mut s = load(data, cid)?;
    let hash = blake3::hash(&bytes).to_hex().to_string();
    if let Some(id) = append {
        let a = s
            .attachments
            .iter()
            .find(|a| a.id == id)
            .ok_or("谱面版本不存在")?;
        if a.format != "images" || k == "pdf" {
            return Err("图片谱面只能追加图片；PDF 请添加为新版本".into());
        }
        if a.pages.len() >= 80 {
            return Err("一份图片谱面最多 80 页".into());
        }
        if a.pages.iter().any(|p| p.id == hash) {
            return Err("这张图片已经在该谱面中".into());
        }
    } else if let Some(a) = s
        .attachments
        .iter()
        .find(|a| a.pages.len() == 1 && a.pages[0].id == hash)
    {
        let id = a.id.clone();
        let dest = asset_path(data, cid, &a.pages[0])?;
        std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
        s.active = Some(id);
        save(data, &s)?;
        return describe(data, &s);
    } else if s.attachments.len() >= 40 {
        return Err("一首曲目最多 40 份谱面附件".into());
    }
    let total: usize = s
        .attachments
        .iter()
        .flat_map(|a| &a.pages)
        .map(|a| a.size)
        .sum();
    if total + bytes.len() > 240_000_000 {
        return Err("这首曲目的谱面附件总量超过 240 MB".into());
    }
    let asset = PageAsset {
        id: hash.clone(),
        name: name.clone(),
        kind: k.clone(),
        size: bytes.len(),
    };
    let dest = asset_path(data, cid, &asset)?;
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    if let Some(id) = append {
        let a = s.attachments.iter_mut().find(|a| a.id == id).unwrap();
        a.pages.push(asset);
        s.active = Some(id.into());
    } else {
        let mut id = hash.clone();
        let mut version = 2;
        while s.attachments.iter().any(|a| a.id == id) {
            id = format!("{hash}-{version}");
            version += 1;
        }

        s.attachments.push(Attachment {
            id: id.clone(),
            name,
            format: if k == "pdf" { "pdf" } else { "images" }.into(),
            pages: vec![asset],
            view: View::default(),
            mapping: None,
            annotations: vec![],
            sources: Default::default(),
        });
        s.active = Some(id);
    }
    save(data, &s)?;
    describe(data, &s)
}
pub fn read(data: &Path, cid: &str, id: &str, page: usize) -> Result<(Vec<u8>, String), String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    let s = load(data, cid)?;
    let a = s
        .attachments
        .iter()
        .find(|a| a.id == id)
        .ok_or("谱面版本不存在")?;
    let p = a.pages.get(page).ok_or("谱页不存在")?;
    let path = asset_path(data, cid, p)?;
    if std::fs::metadata(&path)
        .map_err(|e| format!("谱页不可用：{e}"))?
        .len()
        > MAX_BYTES as u64
    {
        return Err("谱页文件超过 48 MB".into());
    }
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    if blake3::hash(&b).to_hex().as_str() != p.id {
        return Err("谱页文件内容已改变，请重新导入".into());
    }
    Ok((b, p.kind.clone()))
}
pub fn update(
    data: &Path,
    cid: &str,
    id: Option<&str>,
    action: &str,
    name: Option<&str>,
    view: Option<View>,
    page: usize,
    direction: i32,
) -> Result<Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    let mut s = load(data, cid)?;
    if action == "select" {
        if let Some(id) = id {
            if !s.attachments.iter().any(|a| a.id == id) {
                return Err("谱面版本不存在".into());
            }
        }
        s.active = id.map(String::from);
    } else {
        let id = id.ok_or("谱面版本未指定")?;
        let at = s
            .attachments
            .iter()
            .position(|a| a.id == id)
            .ok_or("谱面版本不存在")?;
        match action {
            "rename" => s.attachments[at].name = title(name.ok_or("名称未指定")?)?,
            "remove" => {
                s.attachments.remove(at);
                if s.active.as_deref() == Some(id) {
                    s.active = s.attachments.first().map(|a| a.id.clone());
                }
            }
            "view" => {
                let mut v = view.ok_or("阅读设置未指定")?;
                if v.page == 0
                    || v.page > 5000
                    || v.zoom < 25
                    || v.zoom > 300
                    || ![0, 90, 180, 270].contains(&v.rotation)
                {
                    return Err("阅读设置超出范围".into());
                }
                if s.attachments[at].format == "images" {
                    v.page = v.page.min(s.attachments[at].pages.len());
                }
                s.attachments[at].view = v;
            }
            "movePage" | "removePage" => {
                let a = &mut s.attachments[at];
                if a.format != "images" || page >= a.pages.len() {
                    return Err("图片谱页不存在".into());
                }
                if action == "removePage" {
                    if a.pages.len() == 1 {
                        return Err("最后一页不能移除，请移除整个版本".into());
                    }
                    let removed = a.pages.remove(page);
                    a.annotations.retain(|n| n.asset_id != removed.id);
                    if let Some(m) = &mut a.mapping {
                        m.anchors.retain(|v| v.page != page + 1);
                        for v in &mut m.anchors {
                            if v.page > page + 1 {
                                v.page -= 1;
                            }
                        }
                        if m.anchors.is_empty() {
                            a.mapping = None;
                        }
                    }
                    a.view.page = a.view.page.min(a.pages.len());
                } else {
                    let next = page as i32 + direction;
                    if ![-1, 1].contains(&direction) || next < 0 || next as usize >= a.pages.len() {
                        return Err("谱页已在边界".into());
                    }
                    a.pages.swap(page, next as usize);
                    if let Some(m) = &mut a.mapping {
                        for v in &mut m.anchors {
                            if v.page == page + 1 {
                                v.page = next as usize + 1;
                            } else if v.page == next as usize + 1 {
                                v.page = page + 1;
                            }
                        }
                    }
                    a.view.page = next as usize + 1;
                }
            }
            _ => return Err("谱面操作无效".into()),
        }
    }
    save(data, &s)?;
    describe(data, &s)
}
pub fn summaries(data: &Path) -> std::collections::BTreeMap<String, (usize, Vec<String>)> {
    let mut result = std::collections::BTreeMap::new();
    let Ok(dirs) = std::fs::read_dir(data.join("score-attachments")) else {
        return result;
    };
    for dir in dirs.take(10_000).flatten() {
        let Ok(bytes) = std::fs::read(dir.path().join("manifest.json")) else {
            continue;
        };
        let Ok(store) = serde_json::from_slice::<Store>(&bytes) else {
            continue;
        };
        result.insert(
            store.content_id,
            (
                store.attachments.len(),
                store.attachments.into_iter().map(|a| a.name).collect(),
            ),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn annotation_identity_conflicts_merge_and_page_removal(){
        let root=std::env::temp_dir().join(format!("paper-notes-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));let cid="annotations";
        let s=add(&root,cid,"教师谱",b"\x89PNG\r\n\x1a\nfirst".to_vec(),None).unwrap();let book=s["active"].as_str().unwrap();let asset=s["attachments"][0]["pages"][0]["id"].as_str().unwrap().to_owned();
        let d=AnnotationDraft{asset_id:asset.clone(),page:1,x:0.2,y:0.3,text:"这里换指 3→1".into(),color:"yellow".into(),width:None,height:None,ink:None};
        let s=edit_annotation(&root,cid,book,None,Some(d.clone()),None).unwrap();let note:Annotation=serde_json::from_value(s["attachments"][0]["annotations"][0].clone()).unwrap();
        let mut changed=d.clone();changed.text="放松手腕，保持连奏".into();changed.width=Some(0.4);changed.height=Some(0.1);edit_annotation(&root,cid,book,Some(&note.id),Some(changed.clone()),Some(note.clone())).unwrap();
        assert!(edit_annotation(&root,cid,book,Some(&note.id),Some(d.clone()),Some(note.clone())).is_err());let before=list(&root,cid).unwrap();let mut lone=changed.clone();lone.height=None;assert!(edit_annotation(&root,cid,book,None,Some(lone),None).is_err());let mut wide=changed.clone();wide.width=Some(0.9);assert!(edit_annotation(&root,cid,book,None,Some(wide),None).is_err());let mut bad=d.clone();bad.x=2.;assert!(edit_annotation(&root,cid,book,None,Some(bad),None).is_err());assert_eq!(list(&root,cid).unwrap(),before);
        add(&root,cid,"二页",b"\x89PNG\r\n\x1a\nsecond".to_vec(),Some(book)).unwrap();update(&root,cid,Some(book),"movePage",None,None,0,1).unwrap();assert_eq!(list(&root,cid).unwrap()["attachments"][0]["annotations"][0]["assetId"],asset);
        let mut files=std::collections::BTreeMap::new();let (papers,active)=export_package(&root,cid,&mut files).unwrap();let target=root.join("target");import_package(&target,cid,&papers,active,&files,"merge").unwrap();import_package(&target,cid,&papers,active,&files,"merge").unwrap();assert_eq!(list(&target,cid).unwrap()["attachments"][0]["annotations"].as_array().unwrap().len(),1);assert_eq!(list(&target,cid).unwrap()["attachments"][0]["annotations"][0]["width"],serde_json::json!(0.4_f32));
        update(&root,cid,Some(book),"removePage",None,None,1,0).unwrap();assert!(list(&root,cid).unwrap()["attachments"][0].get("annotations").is_none());
    }
    #[test]
    fn ink_page_transaction_conflicts_undo_package_and_transfer(){
        let root=std::env::temp_dir().join(format!("paper-ink-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));let cid="ink-test";
        let initial=add(&root,cid,"教师谱",b"\x89PNG\r\n\x1a\nink-page".to_vec(),None).unwrap();let book=initial["active"].as_str().unwrap().to_owned();let asset=initial["attachments"][0]["pages"][0]["id"].as_str().unwrap().to_owned();
        edit_annotation(&root,cid,&book,None,Some(AnnotationDraft{asset_id:asset.clone(),page:1,x:0.2,y:0.2,text:"已有提示".into(),color:"blue".into(),width:None,height:None,ink:None}),None).unwrap();
        let mark=Annotation{id:"ink-a".into(),asset_id:asset.clone(),page:1,x:0.1,y:0.2,width:Some(0.3),height:Some(0.2),text:"".into(),color:"red".into(),ink:Some(InkStroke{points:vec![[0.,0.],[1.,1.]],thickness:0.0022})};let mut second=mark.clone();second.id="ink-b".into();second.y=0.6;
        let strokes=vec![mark.clone(),second.clone()];let saved=edit_ink(&root,cid,&book,&asset,1,vec![],strokes.clone()).unwrap();assert_eq!(saved["attachments"][0]["annotations"].as_array().unwrap().len(),3);
        assert!(edit_ink(&root,cid,&book,&asset,1,vec![],vec![]).is_err());let mut bad=mark.clone();bad.ink.as_mut().unwrap().points[1][0]=2.;assert!(edit_ink(&root,cid,&book,&asset,1,strokes.clone(),vec![bad]).is_err());assert_eq!(list(&root,cid).unwrap()["attachments"][0]["annotations"].as_array().unwrap().len(),3);
        edit_ink(&root,cid,&book,&asset,1,strokes.clone(),vec![]).unwrap();let erased=list(&root,cid).unwrap();assert_eq!(erased["attachments"][0]["annotations"][0]["text"],"已有提示");edit_ink(&root,cid,&book,&asset,1,vec![],strokes.clone()).unwrap();
        let mut files=std::collections::BTreeMap::new();let (papers,active)=export_package(&root,cid,&mut files).unwrap();let target=root.join("import");import_package(&target,cid,&papers,active,&files,"replace").unwrap();assert_eq!(list(&target,cid).unwrap()["attachments"][0]["annotations"][1]["ink"]["points"],json!([[0.,0.],[1.,1.]]));
        let dest=add(&root,cid,"新版谱",b"\x89PNG\r\n\x1a\nnew-ink-page".to_vec(),None).unwrap();let dest_book=dest["active"].as_str().unwrap().to_owned();let dest_asset=dest["attachments"].as_array().unwrap().iter().find(|a|a["id"]==dest_book).unwrap()["pages"][0]["id"].as_str().unwrap().to_owned();let preview=preview_transfer(&root,cid,&book,&dest_book).unwrap();
        apply_transfer(&root,cid,PaperTransfer{source:book,target:dest_book.clone(),baseline:preview["baseline"].as_str().unwrap().into(),annotations:vec![TransferAnnotation{id:mark.id,asset_id:dest_asset,page:1,x:0.3,y:0.4}],anchors:vec![],replace_anchors:false}).unwrap();let after=list(&root,cid).unwrap();let copied=&after["attachments"].as_array().unwrap().iter().find(|a|a["id"]==dest_book).unwrap()["annotations"][0];assert_eq!(copied["ink"]["points"],json!([[0.,0.],[1.,1.]]));assert_eq!(copied["x"].as_f64().unwrap() as f32,0.3f32);
    }
    #[test]
    fn mapping_reorder_remove_and_package_keep_page_identity() {
        let root = std::env::temp_dir().join(format!(
            "paper-map-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cid = "mapping-score";
        let first = add(&root, cid, "谱页一", b"\x89PNG\r\n\x1a\none".to_vec(), None).unwrap();
        let id = first["active"].as_str().unwrap();
        add(
            &root,
            cid,
            "谱页二",
            b"\x89PNG\r\n\x1a\ntwo".to_vec(),
            Some(id),
        )
        .unwrap();
        let map = Mapping {
            grid: "grid-v1-test".into(),
            enabled: true,
            anchors: vec![
                PageAnchor {
                    measure: 1,
                    page: 1, region:Some(PageRegion{x:0.2,y:0.3,width:0.4,height:0.1}),
                },
                PageAnchor {
                    measure: 5,
                    page: 2, region:None,
                },
                PageAnchor {
                    measure: 9,
                    page: 1, region:None,
                },
            ],
        };
        let mut invalid=map.clone();invalid.anchors[0].region.as_mut().unwrap().width=0.9;
        assert!(set_mapping(&root,cid,id,Some(invalid)).is_err());
        let old:PageAnchor=serde_json::from_value(json!({"measure":1,"page":1})).unwrap();assert!(old.region.is_none());
        set_mapping(&root, cid, id, Some(map.clone())).unwrap();
        assert_eq!(
            list(&root, cid).unwrap()["attachments"][0]["mapping"]["anchors"][2]["page"],
            1
        );
        let bad = Mapping {
            anchors: vec![PageAnchor {
                measure: 1,
                page: 3, region:None,
            }],
            ..map.clone()
        };
        assert!(set_mapping(&root, cid, id, Some(bad)).is_err());
        update(&root, cid, Some(id), "movePage", None, None, 0, 1).unwrap();
        let data = list(&root, cid).unwrap();
        assert_eq!(data["attachments"][0]["mapping"]["anchors"][0]["page"], 2);
        assert_eq!(data["attachments"][0]["mapping"]["anchors"][1]["page"], 1);
        assert_eq!(data["attachments"][0]["mapping"]["anchors"][0]["region"]["width"].as_f64().unwrap() as f32,0.4f32);
        let mut files = std::collections::BTreeMap::new();
        let (papers, active) = export_package(&root, cid, &mut files).unwrap();
        let target = root.join("import");
        import_package(&target, cid, &papers, active, &files, "replace").unwrap();
        assert_eq!(
            list(&target, cid).unwrap()["attachments"][0]["mapping"]["grid"],
            "grid-v1-test"
        );
        update(&root, cid, Some(id), "removePage", None, None, 0, 0).unwrap();
        let data = list(&root, cid).unwrap();
        let a = data["attachments"][0]["mapping"]["anchors"]
            .as_array()
            .unwrap();
        assert_eq!(a.len(), 2);
        assert!(a.iter().all(|a| a["page"] == 1));
    }
    #[test]
    fn copies_reading_state_reordering_and_identity_are_durable() {
        let root = std::env::temp_dir().join(format!(
            "score-attachments-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let cid = "piano-score";
        let b = b"\x89PNG\r\n\x1a\nfirst".to_vec();
        let s = add(&root, cid, "第一页", b.clone(), None).unwrap();
        let id = s["active"].as_str().unwrap();
        assert_eq!(read(&root, cid, id, 0).unwrap().0, b);
        assert!(read(&root, "another", id, 0).is_err());
        let s = add(
            &root,
            cid,
            "第二页",
            b"\x89PNG\r\n\x1a\nsecond".to_vec(),
            Some(id),
        )
        .unwrap();
        assert_eq!(s["attachments"][0]["pages"].as_array().unwrap().len(), 2);
        update(
            &root,
            cid,
            Some(id),
            "view",
            None,
            Some(View {
                page: 2,
                zoom: 150,
                fit: false,
                rotation: 90,
            }),
            0,
            0,
        )
        .unwrap();
        update(&root, cid, Some(id), "movePage", None, None, 1, -1).unwrap();
        assert_eq!(
            list(&root, cid).unwrap()["attachments"][0]["pages"][0]["name"],
            "第二页"
        );
        assert_eq!(
            list(&root, cid).unwrap()["attachments"][0]["view"]["zoom"],
            150
        );
        assert!(add(&root, cid, "错误", b"<svg>script</svg>".to_vec(), None).is_err());
        assert!(add(&root, cid, "PDF", b"%PDF-test".to_vec(), Some(id)).is_err());
        update(&root, cid, Some(id), "remove", None, None, 0, 0).unwrap();
        assert!(
            list(&root, cid).unwrap()["attachments"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            folder(&root, cid)
                .unwrap()
                .join(format!("{}.png", blake3::hash(&b).to_hex()))
                .exists()
        );
    }
}

pub fn export_package(
    data: &Path,
    cid: &str,
    files: &mut std::collections::BTreeMap<String, Vec<u8>>,
) -> Result<(Vec<crate::piece_package::Paper>, Option<usize>), String> {
    let store = load(data, cid)?;
    let active = store
        .attachments
        .iter()
        .position(|a| Some(&a.id) == store.active.as_ref());
    let mut papers = vec![];
    for (i, a) in store.attachments.iter().enumerate() {
        let mut pages = vec![];
        for (j, p) in a.pages.iter().enumerate() {
            let bytes = std::fs::read(asset_path(data, cid, p)?)
                .map_err(|e| format!("谱页 {} 无法打包：{e}", p.name))?;
            if blake3::hash(&bytes).to_hex().as_str() != p.id {
                return Err(format!("谱页 {} 校验失败", p.name));
            }
            let path = format!("papers/{i}/{j}.{}", p.kind);
            pages.push(crate::piece_package::asset(
                path.clone(),
                p.name.clone(),
                &bytes,
            ));
            files.insert(path, bytes);
        }
        papers.push(crate::piece_package::Paper {
            name: a.name.clone(),
            format: a.format.clone(),
            pages,
            view: a.view.clone(),
            mapping: a.mapping.clone(),
            annotations: a.annotations.clone(),
        });
    }
    Ok((papers, active))
}
pub fn validate_package(
    papers: &[crate::piece_package::Paper],
    active: Option<usize>,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    if papers.len() > 40 || active.is_some_and(|i| i >= papers.len()) {
        return Err("曲目包谱页目录无效".into());
    }
    let mut total = 0usize;
    for p in papers {
        title(&p.name)?;
        validate_annotations(
            &p.annotations,
            &p.format,
            &p.pages.iter().map(|a| a.hash.clone()).collect::<Vec<_>>(),
        )?;
        if let Some(m) = &p.mapping {
            m.validate(&p.format, p.pages.len())?;
        }
        if p.pages.is_empty()
            || p.pages.len() > 80
            || !["pdf", "images"].contains(&p.format.as_str())
            || (p.format == "pdf" && p.pages.len() != 1)
            || p.view.page == 0
            || p.view.page > 5000
            || !(25..=300).contains(&p.view.zoom)
            || ![0, 90, 180, 270].contains(&p.view.rotation)
            || (p.format == "images" && p.view.page > p.pages.len())
        {
            return Err("曲目包谱页或阅读设置无效".into());
        }
        for a in &p.pages {
            title(&a.name)?;
            let bytes = files.get(&a.path).ok_or("曲目包缺少谱页")?;
            let k = kind(bytes)?;
            if !a.path.starts_with("papers/")
                || (p.format == "pdf") != (k == "pdf")
                || !a.path.ends_with(&format!(".{k}"))
            {
                return Err("曲目包谱页类型不一致".into());
            }
            total = total.checked_add(bytes.len()).ok_or("谱页大小无效")?;
        }
    }
    if total > 240_000_000 {
        return Err("曲目包谱页超过 240 MB".into());
    }
    Ok(())
}
pub fn import_package(
    data: &Path,
    cid: &str,
    papers: &[crate::piece_package::Paper],
    active: Option<usize>,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    policy: &str,
) -> Result<(), String> {
    validate_package(papers, active, files)?;
    let mut store = load(data, cid)?;
    if policy == "replace" {
        store.attachments.clear();
        store.active = None;
    }
    let mut incoming = vec![];
    let mut blobs = vec![];
    for p in papers {
        let pages: Vec<PageAsset> = p
            .pages
            .iter()
            .map(|a| PageAsset {
                id: a.hash.clone(),
                name: a.name.clone(),
                kind: kind(&files[&a.path]).unwrap().into(),
                size: a.size,
            })
            .collect();
        let existing = store
            .attachments
            .iter()
            .find(|a| {
                a.name == p.name
                    && a.format == p.format
                    && a.pages
                        .iter()
                        .map(|p| &p.id)
                        .eq(pages.iter().map(|p| &p.id))
            })
            .map(|a| a.id.clone());
        let id = if let Some(id) = existing {
            let local = store.attachments.iter_mut().find(|a| a.id == id).unwrap();
            for note in &p.annotations {
                if !local.annotations.iter().any(|n| {
                    n.id == note.id
                        || (n.asset_id == note.asset_id
                            && n.page == note.page
                            && n.x == note.x
                            && n.y == note.y
                            && n.text == note.text
                            && n.color == note.color)
                }) {
                    local.annotations.push(note.clone());
                }
            }
            validate_annotations(
                &local.annotations,
                &local.format,
                &local.pages.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
            )?;
            id
        } else {
            let signature = serde_json::to_vec(&pages).map_err(|e| e.to_string())?;
            let base = blake3::hash(&signature).to_hex().to_string();
            let mut id = base.clone();
            let mut suffix = 1;
            while store.attachments.iter().any(|a| a.id == id) {
                suffix += 1;
                id = format!("{base}-{suffix}");
            }
            store.attachments.push(Attachment {
                id: id.clone(),
                name: p.name.clone(),
                format: p.format.clone(),
                pages: pages.clone(),
                view: p.view.clone(),
                mapping: p.mapping.clone(),
                annotations: p.annotations.clone(),
                sources: Default::default(),
            });
            id
        };
        for (a, page) in p.pages.iter().zip(&pages) {
            blobs.push((asset_path(data, cid, page)?, &files[&a.path]));
        }
        incoming.push(id);
    }
    if store.attachments.len() > 40
        || store
            .attachments
            .iter()
            .flat_map(|a| &a.pages)
            .map(|p| p.size)
            .sum::<usize>()
            > 240_000_000
    {
        return Err("合并后谱页超出上限，请整理后再导入".into());
    }
    if store.active.is_none() {
        store.active = active
            .and_then(|i| incoming.get(i).cloned())
            .or_else(|| store.attachments.first().map(|a| a.id.clone()));
    }
    std::fs::create_dir_all(folder(data, cid)?).map_err(|e| e.to_string())?;
    for (path, bytes) in blobs {
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    save(data, &store)
}

pub fn list_verified(data: &Path, cid: &str) -> Result<Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    let store = load(data, cid)?;
    let mut v = describe(data, &store)?;
    for (book, item) in store
        .attachments
        .iter()
        .zip(v["attachments"].as_array_mut().unwrap())
    {
        for (asset, page) in book.pages.iter().zip(item["pages"].as_array_mut().unwrap()) {
            let path = asset_path(data, cid, asset)?;
            let status = if std::fs::metadata(&path).is_ok_and(|m| m.len() != asset.size as u64) {
                "changed"
            } else {
                match std::fs::read(path) {
                    Ok(bytes) => {
                        if blake3::hash(&bytes).to_hex().as_str() == asset.id {
                            "ready"
                        } else {
                            "changed"
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            "missing"
                        } else {
                            "unreadable"
                        }
                    }
                }
            };
            page["status"] = json!(status);
            page["available"] = json!(status == "ready");
        }
    }
    Ok(v)
}

pub fn repair(
    data: &Path,
    cid: &str,
    id: &str,
    page: usize,
    bytes: Vec<u8>,
) -> Result<Value, String> {
    let k = kind(&bytes)?;
    let lock = crate::library_workspace::lock_for(data)?;
    let _g = lock.lock().map_err(|_| "谱页目录正忙")?;
    let s = load(data, cid)?;
    let a = s
        .attachments
        .iter()
        .find(|a| a.id == id)
        .ok_or("待修复谱面不存在")?;
    let asset = a.pages.get(page).ok_or("待修复谱页不存在")?;
    if asset.kind != k
        || asset.size != bytes.len()
        || blake3::hash(&bytes).to_hex().as_str() != asset.id
    {
        return Err("文件内容与原谱页不符；请选择原文件修复，或添加为新版本".into());
    }
    let path = asset_path(data, cid, asset)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("{}.repair", asset.kind));
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())?;
    Ok(json!({"updated":true,"id":id,"page":page}))
}
