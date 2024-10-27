use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat, Rgba, RgbaImage, imageops::FilterType};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use rusttype::{Font, Scale};
use std::error::Error;
use std::path::Path;
use std::io::Cursor;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TextInfo {
    text: String,
    size: f32,
    color: [u8; 4],
    x_offset: i32,  // 添加x偏移量
    y_offset: i32,  // 添加y偏移量
}

#[derive(Serialize, Deserialize)]
pub struct LogoInfo {
    width: u32,
    height: u32,
    x_offset: i32,
    y_offset: i32,
}

#[derive(Serialize, Deserialize)]
pub struct BlurConfig {
    sigma: f32,     // 高斯模糊的强度
    padding: u32,   // 模糊区域的内边距
}

#[derive(Serialize, Deserialize)]
pub struct PolaroidConfig {
    frame_color: [u8; 4],
    frame_padding: u32,
    bottom_height: u32,
    corner_radius: u32,
    left_text: Option<TextInfo>,
    right_text: Option<TextInfo>,
    logo: Option<LogoInfo>,
    blur: Option<BlurConfig>,
    quality: u8,    // JPEG质量设置 (1-100)
}

pub struct ImageProcessor {
    config: PolaroidConfig,
    font: Font<'static>,
    logo: Option<DynamicImage>,
}

impl ImageProcessor {
    pub fn new(config: PolaroidConfig, logo_path: Option<&str>) -> Result<Self, Box<dyn Error>> {
        let font_data = include_bytes!("D:\\Temp\\TemNdk\\12\\previewer\\common\\bin\\fonts\\HarmonyOS_Sans_SC.ttf");
        let font = Font::try_from_bytes(font_data as &[u8])
            .ok_or("Error loading font")?;

        // 加载logo（如果提供了路径）
        let logo = if let Some(path) = logo_path {
            Some(image::open(path)?)
        } else {
            None
        };

        Ok(Self { config, font, logo })
    }

    pub fn process_from_file<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> Result<(), Box<dyn Error>> {
        let img = image::open(input_path)?;
        let processed = self.process_image(img)?;
        
        // 使用高质量设置保存图片
        if output_path.as_ref().extension().map_or(false, |ext| ext == "jpg" || ext == "jpeg") {
            processed.save_with_format(output_path, ImageFormat::Jpeg)?;
        } else {
            processed.save(output_path)?;
        }
        Ok(())
    }

    pub fn process_from_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let img = image::load_from_memory(bytes)?;
        let processed = self.process_image(img)?;
        
        let mut cursor = Cursor::new(Vec::new());
        if self.config.quality < 100 {
            processed.write_to(&mut cursor, ImageFormat::Jpeg)?;
        } else {
            processed.write_to(&mut cursor, ImageFormat::Png)?;
        }
        Ok(cursor.into_inner())
    }

    fn process_image(&self, img: DynamicImage) -> Result<DynamicImage, Box<dyn Error>> {
        let (width, height) = img.dimensions();
        
        // 计算新尺寸
        let new_width = width + self.config.frame_padding * 2;
        let new_height = height + self.config.frame_padding * 2 + self.config.bottom_height;
        
        // 创建新图片
        let mut new_image: RgbaImage = ImageBuffer::new(new_width, new_height);

        // 如果启用了模糊效果
        if let Some(blur_config) = &self.config.blur {
            // 创建放大的模糊背景
            let mut blur_img = img.resize(
                width + blur_config.padding * 2,
                height + blur_config.padding * 2,
                FilterType::Lanczos3
            );
            
            // 应用高斯模糊
            blur_img = blur_img.blur(blur_config.sigma);
            
            // 将模糊背景放置到新图片中
            image::imageops::overlay(
                &mut new_image,
                &blur_img.to_rgba8(),
                (self.config.frame_padding - blur_config.padding) as i64,
                (self.config.frame_padding - blur_config.padding) as i64
            );
        }

        // 绘制白色背景框
        self.draw_rounded_rectangle(&mut new_image, 0, 0, new_width, new_height, self.config.corner_radius)?;

        // 复制原图到中央（保持原始质量）
        image::imageops::overlay(
            &mut new_image,
            &img.to_rgba8(),
            self.config.frame_padding as i64,
            self.config.frame_padding as i64
        );

        // 添加底部信息
        self.add_bottom_info(&mut new_image)?;

        Ok(DynamicImage::ImageRgba8(new_image))
    }

    fn draw_rounded_rectangle(
        &self,
        image: &mut RgbaImage,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
    ) -> Result<(), Box<dyn Error>> {
        // 填充主体区域
        draw_filled_rect_mut(
            image,
            imageproc::rect::Rect::at(x as i32, y as i32).of_size(width, height),
            Rgba(self.config.frame_color),
        );

        if radius > 0 {
            for dx in 0..radius {
                for dy in 0..radius {
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();
                    if distance > radius as f32 {
                        for corner in &[(0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1)] {
                            if let Some(pixel) = image.get_pixel_mut_checked(
                                x + corner.0 + if corner.0 == 0 { dx } else { radius - dx },
                                y + corner.1 + if corner.1 == 0 { dy } else { radius - dy }
                            ) {
                                *pixel = Rgba([0, 0, 0, 0]);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn add_bottom_info(&self, image: &mut RgbaImage) -> Result<(), Box<dyn Error>> {
        let bottom_y = image.height() - self.config.bottom_height;

        // 添加左侧文字
        if let Some(text_info) = &self.config.left_text {
            let scale = Scale {
                x: text_info.size,
                y: text_info.size,
            };

            draw_text_mut(
                image,
                Rgba(text_info.color),
                self.config.frame_padding as i32 + text_info.x_offset,
                bottom_y as i32 + text_info.y_offset,
                scale,
                &self.font,
                &text_info.text,
            );
        }

        // 添加右侧文字
        if let Some(text_info) = &self.config.right_text {
            let scale = Scale {
                x: text_info.size,
                y: text_info.size,
            };

            let text_width = self.font.layout(&text_info.text, scale, Default::default())
                .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
                .last()
                .unwrap_or(0.0) as u32;

            draw_text_mut(
                image,
                Rgba(text_info.color),
                image.width() as i32 - text_width as i32 - self.config.frame_padding as i32 + text_info.x_offset,
                bottom_y as i32 + text_info.y_offset,
                scale,
                &self.font,
                &text_info.text,
            );
        }

        // 添加Logo
        if let (Some(logo_info), Some(logo)) = (&self.config.logo, &self.logo) {
            let resized_logo = logo.resize(
                logo_info.width,
                logo_info.height,
                FilterType::Lanczos3
            );
            
            image::imageops::overlay(
                image,
                &resized_logo.to_rgba8(),
                ((image.width() as i32 - logo_info.width as i32) / 2 + logo_info.x_offset) as i64,
                (bottom_y as i32 + logo_info.y_offset) as i64
            );
        }

        Ok(())
    }
}

// 使用示例
fn main() -> Result<(), Box<dyn Error>> {
    let config = PolaroidConfig {
        frame_color: [255, 255, 255, 255],
        frame_padding: 40,
        bottom_height: 80,
        corner_radius: 20,
        left_text: Some(TextInfo {
            text: "AF 56/1.7 XF".to_string(),
            size: 20.0,
            color: [0, 0, 0, 255],
            x_offset: 0,
            y_offset: 20,
        }),
        right_text: Some(TextInfo {
            text: "56mm f/2.8 1/50s ISO4000".to_string(),
            size: 16.0,
            color: [0, 0, 0, 255],
            x_offset: 0,
            y_offset: 20,
        }),
        logo: Some(LogoInfo {
            width: 100,
            height: 40,
            x_offset: 0,
            y_offset: 20,
        }),
        blur: Some(BlurConfig {
            sigma: 8.0,
            padding: 200,
        }),
        quality: 100, // 最高质量
    };

    let processor = ImageProcessor::new(config, Some("fujifilm.png"))?;
    processor.process_from_file("input.jpg", "output2.jpg")?;

    Ok(())
}