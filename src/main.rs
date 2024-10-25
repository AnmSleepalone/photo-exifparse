use nom_exif::*;
use std::fs::File;
use std::io::Write;
use uuid::Uuid;
use std::fs;
use std::collections::HashMap;

fn bytes_to_temp_file(bytes: &[u8],extension: &str) -> Result<String> {
    // 创建临时文件
    let temp_dir = std::env::temp_dir();
    let mut  temp_path = temp_dir.join(format!("temp_{}", Uuid::new_v4()));
    temp_path.set_extension(extension);
    let mut temp_file = File::create(&temp_path)?;
    // 写入字节数据
    temp_file.write_all(bytes)?;
    Ok(temp_path.to_string_lossy().into_owned())
}

fn main() -> Result<()> {
    // 测试用本地文件案例
    // 打开文件并读取内容
    // 读取文件内容
    let contents = fs::read_to_string("C:\\Users\\Administrator\\Desktop\\bb2.txt")
        .expect("无法读取文件");
    // 解析JSON到HashMap
    let json_map: HashMap<String, i32> = serde_json::from_str(&contents).unwrap();
    // 找到最大的键值以确定数组大小
    let max_key = json_map.keys()
        .map(|k| k.parse::<usize>().unwrap())
        .max()
        .unwrap();
    //  创建无符号字节数组
     let mut bytes = vec![0u8; max_key + 1];
    //  填充字节数组
     for (key, value) in json_map {
         let index = key.parse::<usize>().unwrap();
         // 将i32值转换为u8
         bytes[index] = value as u8;
     }
    let file_str:String = bytes_to_temp_file(&bytes,"jpg")?;
    let mut parser = MediaParser::new();
    let ms = MediaSource::file_path(file_str)?;
    if ms.has_exif() {
        // Parse the file as an Exif-compatible file
        let iter: ExifIter = parser.parse(ms)?;
        for item in iter {
            println!("{:?}", item);
        }
    } else if ms.has_track() {
        // Parse the file as a track
        let info: TrackInfo = parser.parse(ms)?;
        info.iter().for_each(|(tag, value)| {
            println!("{:?} = {:?}", tag, value);
        });
    }
    Ok(())
}

// fn main() {
//     // let bytes = include_bytes!("../../test.jpg");
//     // exif_parse(bytes).unwrap();
// }
