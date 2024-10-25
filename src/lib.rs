use napi_derive_ohos::napi;
use napi_ohos::{Error, JsError, Status};
use nom_exif::*;
use serde_json::json;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::result::Result;
use uuid::Uuid;

#[napi]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[napi]
fn getfilepath() -> String {
    let temp_dir = std::env::temp_dir();
    let mut temp_path = temp_dir.join(format!("temp_{}", Uuid::new_v4()));
    temp_path.set_extension("jpg");
    temp_path.to_string_lossy().into_owned()
}

#[napi]
fn bytes_to_temp_file(bytes: &[u8], extension: String) -> Result<String, Error> {
    // 创建临时文件
    let temp_dir = std::env::temp_dir();
    let mut temp_path = temp_dir.join(format!("temp_{}", Uuid::new_v4()));
    temp_path.set_extension(extension);
    let mut temp_file = File::create(&temp_path)?;
    // 写入字节数据
    temp_file.write_all(bytes)?;
    Ok(temp_path.to_string_lossy().into_owned())
}

#[napi]
fn exif_parse(bytes: &[u8], extension: String) -> Result<String, JsError> {
    let file_str: String = bytes_to_temp_file(&bytes, extension)
        .map_err(|e| JsError::from(Error::new(Status::GenericFailure, format!("{:?}", e))))?;
    let mut parser = MediaParser::new();
    let ms = MediaSource::file_path(file_str.clone())
        .map_err(|e| JsError::from(Error::new(Status::GenericFailure, format!("{:?}", e))))?;
    // 用来存储 JSON 数据
    let mut result = json!({ "type": "unknown", "filePath": file_str });
    if ms.has_exif() {
        // Parse the file as an Exif-compatible file
        let iter: ExifIter = parser
            .parse(ms)
            .map_err(|e| JsError::from(Error::new(Status::GenericFailure, format!("{:?}", e))))?;
        let mut exif_data = HashMap::new();
        for item in iter {
            println!("{:?}", item);
            // 通过模式匹配安全解包，并处理未识别标签或空白字符串
            if let (Some(tag), Some(value)) = (item.tag(), item.get_value()) {
                let tag_str = tag.to_string();
                let value_str = value.to_string().trim().to_string();

                // 跳过未识别的标签和空值
                if tag_str.contains("Unrecognized") || value_str.is_empty() {
                    eprintln!("Skipping unrecognized tag or empty value: {:?}", item);
                    continue;
                }

                // 插入数据，并检查是否有重复键值
                if let Some(old_value) = exif_data.insert(tag_str.clone(), value_str.clone()) {
                    eprintln!(
                        "Duplicate tag {} detected. Old value: '{}', New value: '{}'",
                        tag_str, old_value, value_str
                    );
                }
                exif_data.insert(tag_str, value_str);
            } else {
                eprintln!("Skipping item with missing tag or value: {:?}", item);
            }
        }
        result = json!({ "type": "exif", "filePath":file_str, "data": exif_data });
    } else if ms.has_track() {
        // Parse the file as a track
        let info: TrackInfo = parser
            .parse(ms)
            .map_err(|e| JsError::from(Error::new(Status::GenericFailure, format!("{:?}", e))))?;
        info.iter().for_each(|(tag, value)| {
            println!("{:?} = {:?}", tag, value);
        });
        let track_data: HashMap<_, _> = info
            .iter()
            .map(|(tag, value)| (tag.to_string(), value.to_string()))
            .collect();
        result = json!({ "type": "track", "filePath":file_str,"data": track_data });
    }
    Ok(result.to_string())
}
