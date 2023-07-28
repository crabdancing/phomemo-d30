use std::{fs, path::PathBuf, str::FromStr};

use bluetooth_serial_port_async::BtAddr;
use image::{DynamicImage, ImageBuffer, Rgba};
use rusttype::{Font, Scale};

use dimensions::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use snafu::{whatever, OptionExt, ResultExt, Whatever};
// These values are based on those used in polskafan's phomemo_d30 code, available here:
// https://github.com/polskafan/phomemo_d30
pub const INIT_BASE_FLAT: &[u8] = &[
    31, 17, 56, // 1f1138
    31, 17, 18, 31, 17, 19, // 1f11121f1113
    31, 17, 9, // 1f1109
    31, 17, 17, // 1f1111
    31, 17, 25, // 1f1119
    31, 17, 7, // 1f1107
    31, 17, 10, 31, 17, 2, 2, // 1f110a1f110202
];

pub const IMG_PRECURSOR: &[u8] = &[31, 17, 36, 0, 27, 64, 29, 118, 48, 0, 12, 0, 64, 1]; // 1f1124001b401d7630000c004001

const COLOR_BLACK: image::Rgba<u8> = image::Rgba([255u8, 255u8, 255u8, 255u8]);

pub fn generate_image(text: &str, font_scale: f32) -> Result<DynamicImage, Whatever> {
    let dim = Dimensions::new(320, 96);
    dbg!(&dim);
    let font = Vec::from(include_bytes!("DejaVuSans.ttf") as &[u8]);
    let font = Font::try_from_vec(font).with_whatever_context(|| "Failed to parse font data")?;
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

    Ok(canvas)
}

pub fn pack_image(image: &DynamicImage) -> Vec<u8> {
    // This section of code is heavily based on logic from polskafan's phomemo_d30 code on Github
    // See here: https://github.com/polskafan/phomemo_d30
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

// MacAddr

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacAddr([u8; 6]);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrinterAddr {
    MacAddr(MacAddr),
    PrinterName(String),
}

impl Into<String> for MacAddr {
    fn into(self) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl Into<String> for PrinterAddr {
    fn into(self) -> String {
        match self {
            PrinterAddr::MacAddr(addr) => addr.into(),
            PrinterAddr::PrinterName(name) => name,
        }
    }
}

impl FromStr for MacAddr {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16)?;
        }
        Ok(MacAddr(bytes))
    }
}

impl FromStr for PrinterAddr {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(":") {
            Ok(PrinterAddr::MacAddr(MacAddr::from_str(s)?))
        } else {
            Ok(PrinterAddr::PrinterName(s.to_string()))
        }
    }
}

impl Into<BtAddr> for MacAddr {
    fn into(self) -> BtAddr {
        BtAddr(self.0)
    }
}

// D30Config

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct D30Config {
    default: PrinterAddr,
    name_to_mac: IndexMap<String, MacAddr>,
}

impl D30Config {
    pub fn load_toml(path: &PathBuf) -> Result<Self, Whatever> {
        let contents = fs::read_to_string(path).with_whatever_context(|_| "")?;
        Ok(toml::from_str(contents.as_str())
            .with_whatever_context(|_| "Failed to serialize TOML D30 config.")?)
    }

    pub fn read_d30_config() -> Result<D30Config, Whatever> {
        let phomemo_lib_path = xdg::BaseDirectories::with_prefix("phomemo-library")
            .with_whatever_context(|_| "Could not find XDG path with prefix")?;
        let config_path = phomemo_lib_path
            .place_config_file("phomemo-config.toml")
            .with_whatever_context(|_| "Could not place config file")?;
        Ok(D30Config::load_toml(&config_path).with_whatever_context(|_| "Could not load TOML")?)
    }

    pub fn get_mac(&self, printer_addr: &PrinterAddr) -> Result<MacAddr, Whatever> {
        match printer_addr {
            PrinterAddr::MacAddr(addr) => Ok(addr.clone()),
            PrinterAddr::PrinterName(name) => {
                let mac = self.name_to_mac.get(name).with_whatever_context(|| "")?;
                Ok(mac.clone())
            }
        }
    }
}
