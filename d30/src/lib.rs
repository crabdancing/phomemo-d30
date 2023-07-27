use std::io::Write;

use image::{DynamicImage, ImageBuffer, Rgba};
use rusttype::{Font, Scale};

use dimensions::*;
use snafu::{ResultExt, Whatever};

pub const INIT_BASE: &[&[u8]] = &[
    &[31, 17, 56],               // 1f1138
    &[31, 17, 18, 31, 17, 19],   // 1f11121f1113
    &[31, 17, 9],                // 1f1109
    &[31, 17, 17],               // 1f1111
    &[31, 17, 25],               // 1f1119
    &[31, 17, 7],                // 1f1107
    &[31, 17, 10, 31, 17, 2, 2], // 1f110a1f110202
];

pub const IMG_PRECURSOR: &[u8] = &[31, 17, 36, 0, 27, 64, 29, 118, 48, 0, 12, 0, 64, 1]; // 1f1124001b401d7630000c004001
const COLOR_BLACK: image::Rgba<u8> = image::Rgba([255u8, 255u8, 255u8, 255u8]);

pub fn init_conn(port: &mut impl Write) -> Result<(), Whatever> {
    for v in INIT_BASE.iter() {
        port.write(v)
            .with_whatever_context(|_| "Failed to write to target")?;
        port.flush()
            .with_whatever_context(|_| "Failed to flush to target")?;
    }
    Ok(())
}

pub fn generate_image(text: &str, font_scale: f32) -> DynamicImage {
    let dim = Dimensions::new(320, 96);
    dbg!(&dim);
    let font = Vec::from(include_bytes!("DejaVuSans.ttf") as &[u8]);
    let font = Font::try_from_vec(font).unwrap();

    let scale = Scale::uniform(font_scale);

    let actual_size: Dimensions = imageproc::drawing::text_size(scale, &font, &text).into();

    let txt_pos = (actual_size - dim) / -2.;

    let mut canvas: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::new(dim.width() as u32, dim.height() as u32);

    imageproc::drawing::draw_text_mut(
        &mut canvas,
        COLOR_BLACK,
        txt_pos.x as i32,
        txt_pos.y as i32,
        scale,
        &font,
        text,
    );

    let canvas = DynamicImage::from(canvas).rotate270();

    canvas
}

pub fn pack_image(image: &DynamicImage) -> Vec<u8> {
    let threshold: u8 = 127;
    let width = image.width() as usize;
    let height = image.height() as usize;

    let mut bit_grid = vec![vec![0u8; width]; height];

    let image = image.to_rgb8();

    let mut output = Vec::new();
    for (x, y, pixel) in image.enumerate_pixels() {
        let (x, y) = (x as usize, y as usize);

        if pixel[0] > threshold {
            bit_grid[y][x] = 1;
        } else {
            bit_grid[y][x] = 0;
        }
    }

    for bit_row in bit_grid {
        for byte_num in 0..(image.width() / 8) {
            let mut byte: u8 = 0;
            for bit_offset in 0..8 {
                let pixel: u8 = bit_row[(byte_num * 8 + bit_offset) as usize];
                // Raw bit manipulation iterates through 0 through 7, and bitshifts the micro-pixels onto a byte 'sandwich',
                // before it gets shipped off to the D30 printer
                byte |= (pixel & 0x01) << (7 - bit_offset);
                // For instance, instead of storing, e.g. 00000001 00000000 00000000 00000001 00000000 00000000 00000000 00000000
                // We can instead send: 10010000
                // This considerably cuts down on the number of bytes needed to send an image,
                // but of course only works if we don't need to send any gradations of pixel color or intensity.
            }
            output.push(byte);
        }
    }
    output
}

// pub fn add(left: usize, right: usize) -> usize {
//     left + right
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
