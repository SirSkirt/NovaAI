//! Untrusted ZIP admission for sequential teaching. This is strictly a teacher
//! input parser: the archive cannot execute code, edit weights or choose replies.
use neural_nova::trainer::prepare;
use serde::Serialize;
use std::collections::HashSet;
use std::io::{Cursor,Read};
use tauri::command;

const MAX_ZIP_BYTES:usize=4*1024*1024;
const MAX_LESSON_BYTES:usize=50_000;
const MAX_LESSONS:usize=40;
const MAX_TOTAL_BYTES:usize=1_600_000;
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ZipLesson{pub name:String,pub text:String,pub training_samples:usize,pub heldout_samples:usize}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct LessonBundle{pub mode:String,pub lessons:Vec<ZipLesson>,pub total_bytes:usize}
pub fn read_bundle(data:&[u8],mode:&str)->Result<LessonBundle,String>{
    if !matches!(mode,"text"|"dialogue"|"conversation"|"reasoning"){
        return Err("Select Text/Language, Dialogue or Conversation first".into());
    }
    if data.is_empty() || data.len()>MAX_ZIP_BYTES{
        return Err("ZIP must contain data and be smaller than 4 MiB".into());
    }
    let mut archive=zip::ZipArchive::new(Cursor::new(data))
        .map_err(|e|format!("Invalid ZIP: {e}"))?;
    if archive.len()>256{return Err("Archive has too many entries (max 256)".into());}
    let mut lessons=Vec::new();
    let mut names=HashSet::new();
    let mut total=0usize;
    for index in 0..archive.len(){
        let mut file=archive.by_index(index).map_err(|e|e.to_string())?;
        if file.is_dir(){continue;}
        let raw_name=file.name().to_string();
        // Reject traversal, absolute paths, alternate separators or duplicates.
        if raw_name.starts_with('/') || raw_name.contains('\\') ||
            raw_name.split('/').any(|segment|segment.is_empty()||segment=="."||segment=="..")
            || raw_name.chars().any(|c|c.is_control()){
            return Err(format!("Unsafe ZIP filename: {raw_name}"));
        }
        if raw_name.starts_with("__MACOSX/") || raw_name.split('/').last().unwrap_or("").starts_with('.') {
            continue;
        }
        if !raw_name.to_ascii_lowercase().ends_with(".txt"){
            return Err(format!("Only .txt lessons are allowed: {raw_name}"));
        }
        if !names.insert(raw_name.to_ascii_lowercase()){
            return Err(format!("Duplicate lesson name in ZIP: {raw_name}"));
        }
        if lessons.len()>=MAX_LESSONS{return Err("Maximum 40 TXT lessons per batch".into());}
        if file.size()>MAX_LESSON_BYTES as u64{
            return Err(format!("{raw_name}: lesson exceeds 50,000 bytes"));
        }
        let mut bytes=Vec::new();
        file.by_ref().take((MAX_LESSON_BYTES+1) as u64)
            .read_to_end(&mut bytes).map_err(|e|format!("{raw_name}: {e}"))?;
        if bytes.len()>MAX_LESSON_BYTES{return Err(format!("{raw_name}: lesson exceeds size limit"));}
        total+=bytes.len();
        if total>MAX_TOTAL_BYTES{return Err("ZIP's total uncompressed text exceeds 1.6 MiB".into());}
        let text=String::from_utf8(bytes).map_err(|_|format!("{raw_name}: invalid UTF-8"))?;
        if !text.is_ascii(){return Err(format!("{raw_name}: Nova currently requires ASCII lessons"));}
        let text=text.replace("\r\n","\n");
        if text.contains('\r'){return Err(format!("{raw_name}: unsupported line endings"));}
        let parsed=prepare(mode,&text).map_err(|error|format!("{raw_name}: {error}"))?;
        lessons.push(ZipLesson{name:raw_name,text,
            training_samples:parsed.train.len(),heldout_samples:parsed.validation.len()});
    }
    if lessons.is_empty(){return Err("No TXT lessons found inside ZIP".into());}
    lessons.sort_by(|a,b|
        a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    Ok(LessonBundle{mode:mode.to_string(),lessons,total_bytes:total})
}
#[command]
pub fn hybrid_zip_preview(bytes:Vec<u8>,mode:String)->Result<LessonBundle,String>{
    read_bundle(&bytes,&mode)
}
#[cfg(test)]
mod tests{
    use super::*;
    use std::io::Write;
    fn make_zip(entries:&[(&str,&str)])->Vec<u8>{
        let mut out=Cursor::new(Vec::new());
        {
            let mut zip=zip::ZipWriter::new(&mut out);
            for (name,text) in entries{
                zip.start_file(*name,zip::write::SimpleFileOptions::default()).unwrap();
                zip.write_all(text.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        out.into_inner()
    }
    fn curriculum()->String{
        (0..8).map(|n|format!("Unique text section {n} describes a new event with different details."))
            .collect::<Vec<_>>().join("\n\n")
    }
    #[test]fn sorted_and_validated_lessons(){
        let c=curriculum();
        let data=make_zip(&[("02_second.txt",&c),("01_first.txt",&c)]);
        let bundle=read_bundle(&data,"text").unwrap();
        assert_eq!(bundle.lessons[0].name,"01_first.txt");
        assert_eq!(bundle.lessons.len(),2);
        assert!(bundle.lessons.iter().all(|x|x.training_samples>0&&x.heldout_samples>0));
    }
    #[test]fn refuse_traversal_and_unsupported_files(){
        let c=curriculum();
        assert!(read_bundle(&make_zip(&[("../evil.txt",&c)]),"text").is_err());
        assert!(read_bundle(&make_zip(&[("script.js","alert('hi')")]),"text").is_err());
        assert!(read_bundle(&make_zip(&[("good.txt",&c)]),"unknown").is_err());
    }
    #[test]fn require_real_distinct_text_sections(){
        let data=make_zip(&[("weak.txt",&"abcde ".repeat(100))]);
        assert!(read_bundle(&data,"text").is_err());
    }
}