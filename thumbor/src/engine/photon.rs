use super::{Engine, SpecTransform};
use crate::pb::*;
use anyhow::Result;
use bytes::Bytes;
use image::{DynamicImage, ImageBuffer, ImageFormat};
use lazy_static::lazy_static;
use photon_rs::{
    effects, filters, multiple, native::open_image_from_bytes, transform, PhotonImage,
};
use std::{convert::TryFrom, io};

lazy_static! {
    // 预先把水印文件加载为静态变量
    static ref WATERMARK: PhotonImage = {

        // 这里你需要把我 github 项目下的对应图片拷贝到你的根目录
        // 在编译的时候 include_bytes! 宏会直接把文件读入编译后的二进制
        let data = include_bytes!("../../rust-logo.png");
        let watermark = open_image_from_bytes(data).unwrap();
        transform::resize(&watermark, 64, 64, transform::SamplingFilter::Nearest)
    };
}

// 我们目前支持 Photon engine
pub struct Photon(PhotonImage);

// 从 Bytes 转换成 Photon 结构
impl TryFrom<Bytes> for Photon {
    type Error = anyhow::Error;

    fn try_from(data: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(open_image_from_bytes(&data)?))
    }
}

impl Engine for Photon {
    fn apply(&mut self, specs: &[Spec]) {
        for spec in specs.iter() {
            match spec.data {
                Some(spec::Data::Crop(ref v)) => self.transform(v),
                Some(spec::Data::Contrast(ref v)) => self.transform(v),
                Some(spec::Data::Filter(ref v)) => self.transform(v),
                Some(spec::Data::Fliph(ref v)) => self.transform(v),
                Some(spec::Data::Flipv(ref v)) => self.transform(v),
                Some(spec::Data::Resize(ref v)) => self.transform(v),
                Some(spec::Data::Watermark(ref v)) => self.transform(v),
                // 对于目前不认识的 spec，不做任何处理
                _ => {}
            }
        }
    }

    fn generate(self, format: ImageFormat) -> Vec<u8> {
        image_to_buf(self.0, format)
    }
}

impl SpecTransform<&Crop> for Photon {
    fn transform(&mut self, op: &Crop) {
        let img = transform::crop(&mut self.0, op.x1, op.y1, op.x2, op.y2);
        self.0 = img;
    }
}

impl SpecTransform<&Contrast> for Photon {
    fn transform(&mut self, op: &Contrast) {
        effects::adjust_contrast(&mut self.0, op.contrast);
    }
}

impl SpecTransform<&Flipv> for Photon {
    fn transform(&mut self, _op: &Flipv) {
        transform::flipv(&mut self.0)
    }
}

impl SpecTransform<&Fliph> for Photon {
    fn transform(&mut self, _op: &Fliph) {
        transform::fliph(&mut self.0)
    }
}

impl SpecTransform<&Filter> for Photon {
    fn transform(&mut self, op: &Filter) {
        match i32::try_into(op.filter) {
            Ok(filter::Filter::Unspecified) => {}
            Ok(f) => filters::filter(&mut self.0, f.to_str().unwrap()),
            Err(_) => {}
        }
    }
}

impl SpecTransform<&Resize> for Photon {
    fn transform(&mut self, op: &Resize) {
        let img = match i32::try_into(op.rtype).unwrap() {
            resize::ResizeType::Normal => transform::resize(
                &mut self.0,
                op.width,
                op.height,
                resize::SampleFilter::try_from(op.filter).unwrap().into(),
            ),
            resize::ResizeType::SeamCarve => {
                transform::seam_carve(&mut self.0, op.width, op.height)
            }
        };
        self.0 = img;
    }
}

impl SpecTransform<&Watermark> for Photon {
    fn transform(&mut self, op: &Watermark) {
        multiple::watermark(&mut self.0, &WATERMARK, op.x, op.y);
    }
}

// photon 库竟然没有提供在内存中对图片转换格式的方法，只好手工实现
fn image_to_buf(img: PhotonImage, format: ImageFormat) -> Vec<u8> {
    let raw_pixels: Vec<u8> = img.get_raw_pixels();
    let width: u32 = img.get_width();
    let height: u32 = img.get_height();

    let img_buffer: ImageBuffer<image::Rgba<u8>, Vec<u8>> = ImageBuffer::from_vec(width, height, raw_pixels).unwrap();
    let dynimage = DynamicImage::ImageRgba8(img_buffer).to_rgb8();
    // 将图像转换为 RGB 格式

    // 创建一个 Cursor<Vec<u8>> 作为写入器
    let mut buffer: Vec<u8> = Vec::new();
    let mut cursor = io::Cursor::new(&mut buffer);

    // 现在写入图像数据到 cursor
    dynimage.write_to(&mut cursor, format).unwrap();
    buffer
}
