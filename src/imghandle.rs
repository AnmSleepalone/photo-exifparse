use image::{DynamicImage, Rgba, RgbaImage, ImageOutputFormat};
use imageproc::drawing::{draw_text_mut, draw_hollow_rect_mut};
use imageproc::rect::Rect;
use rusttype::{Font, Scale};
use serde::Deserialize;
use std::fs::File;
use std::io::{Cursor, Read};

#[derive(Deserialize)]
struct WatermarkParams {
    text: Option<String>,
    position: (u32, u32),
    color: (u8, u8, u8, u8),
    font_size: f32,
}

#[derive(Deserialize)]
struct FrameParams {
    thickness: u32,
    color: (u8, u8, u8, u8),
}

#[derive(Deserialize)]
struct EditParams {
    watermark: Option<WatermarkParams>,
    frame: Option<FrameParams>,
}

fn load_image_from_bytes(bytes: &[u8]) -> DynamicImage {
    image::load_from_memory(bytes).expect("读取图片失败")
}

fn add_watermark(image: &mut RgbaImage, params: &WatermarkParams) {
    let font_data = include_bytes!("D:\\Temp\\TemNdk\\12\\previewer\\common\\bin\\fonts\\HarmonyOS_Sans_SC.ttf"); // 替换为你系统的字体路径
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
    let scale = Scale { x: params.font_size, y: params.font_size };
    let color = Rgba(params.color);

    draw_text_mut(image, color, params.position.0, params.position.1, scale, &font, &params.text.as_ref().unwrap());
}

fn add_frame(image: &mut RgbaImage, params: &FrameParams) {
    let rect = Rect::at(0, 0).of_size(image.width(), image.height());
    let color = Rgba(params.color);

    for i in 0..params.thickness {
        draw_hollow_rect_mut(image, rect.shrink(i as i32), color);
    }
}

fn process_image(mut image: DynamicImage, params: EditParams) -> Vec<u8> {
    let mut rgba_image = image.to_rgba8();

    // 添加水印
    if let Some(watermark_params) = params.watermark {
        add_watermark(&mut rgba_image, &watermark_params);
    }

    // 添加相框
    if let Some(frame_params) = params.frame {
        add_frame(&mut rgba_image, &frame_params);
    }

    // 将图片保存为字节组
    let mut buffer = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(rgba_image)
        .write_to(&mut buffer, ImageOutputFormat::Png)
        .expect("保存图片失败");

    buffer.into_inner()
}

fn main() {
    // 从文件读取图片
    let mut file = File::open("example.png").expect("打开图片文件失败");
    let mut image_data = Vec::new();
    file.read_to_end(&mut image_data).expect("读取文件失败");

    // 示例 JSON 参数
    let json_params = r#"
    {
        "watermark": {
            "text": "Hello, World!",
            "position": [50, 50],
            "color": [255, 0, 0, 128],
            "font_size": 32.0
        },
        "frame": {
            "thickness": 5,
            "color": [0, 255, 0, 255]
        }
    }"#;

    // 解析 JSON 参数
    let edit_params: EditParams = serde_json::from_str(json_params).expect("解析 JSON 参数失败");

    // 加工图片并返回字节组
    let image = load_image_from_bytes(&image_data);
    let processed_image_bytes = process_image(image, edit_params);

    // 将处理后的图片保存到文件
    std::fs::write("output.png", &processed_image_bytes).expect("保存处理后的图片失败");

    println!("图片处理完成！");
}
