use std::io;
use std::{fs, path::PathBuf, str::FromStr};

use advmac::MacAddr6;
use image::{DynamicImage, ImageBuffer, Rgb};
use log::{trace, warn};
use rusttype::{Font, Scale};

use dimensions::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, Snafu};

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

const COLOR_BLACK: image::Rgb<u8> = Rgb([255u8, 255u8, 255u8]);

#[derive(Debug, Clone, Copy)]
pub enum D30Scale {
    Value(f32),
    Auto { minus: f32 },
}

impl FromStr for D30Scale {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Self::Auto { minus: 0.0 }),
            _ => s
                .parse::<f32>()
                .map(Self::Value)
                .map_err(|_| format!("Invalid value: {}", s)),
        }
    }
}

pub fn generate_image(
    text: &str,
    margins: f32,
    font_scale: D30Scale,
) -> Result<DynamicImage, D30Error> {
    let label_dimensions = Dimensions::new(320, 96);
    trace!("{:#?}", &label_dimensions);
    let font = Vec::from(include_bytes!("DejaVuSans.ttf") as &[u8]);
    let font = Font::try_from_vec(font).context(CouldNotInitFontSnafu)?;
    // let scale = Scale::uniform(font_scale);

    let scale = match font_scale {
        D30Scale::Auto { minus } => {
            // let scale = 100.0;
            let actual_size: Dimensions =
                imageproc::drawing::text_size(Scale::uniform(100.0), &font, &text).into();
            let scale_by_x = (label_dimensions.x - 2.0 * margins) / actual_size.x;
            let scale_by_y = (label_dimensions.y - 2.0 * margins) / actual_size.y;
            100.0
                * if scale_by_y > scale_by_x {
                    scale_by_x
                } else {
                    scale_by_y
                }
                - minus
        }
        D30Scale::Value(font_scale) => font_scale,
    };
    let actual_size: Dimensions =
        imageproc::drawing::text_size(Scale::uniform(scale), &font, &text).into();
    let txt_pos = (actual_size - label_dimensions) / -2.;

    let mut canvas: ImageBuffer<Rgb<u8>, _> = ImageBuffer::new(
        label_dimensions.width() as u32,
        label_dimensions.height() as u32,
    );

    imageproc::drawing::draw_text_mut(
        &mut canvas,
        COLOR_BLACK,
        txt_pos.x as i32,
        txt_pos.y as i32,
        Scale::uniform(scale),
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct D30Config {
    pub default_device: Option<String>,
    pub resolution: IndexMap<String, MacAddr6>,
}

#[derive(Debug, Snafu)]
pub enum D30Error {
    #[snafu(display("Failed to read in automatically detected D30 library configuration path"))]
    CouldNotReadFile { source: io::Error },

    #[snafu(display("Could not init font"))]
    CouldNotInitFont,

    #[snafu(display("Failed to serialize TOML D30 config"))]
    CouldNotParse { source: toml::de::Error },
    #[snafu(display("Could not get XDG path"))]
    CouldNotGetXDGPath { source: xdg::BaseDirectoriesError },
    #[snafu(display("Could not place config file"))]
    CouldNotPlaceConfigFile { source: io::Error },
    #[snafu(display("No default device specified"))]
    NoDefaultDevice,

    #[snafu(display("Could not parse MAC address, or find in hostname table: {device}"))]
    CouldNotParseOrLookupMacAddress { device: String },

    #[snafu(display("Could not parse specified device as MAC address:\n"))]
    CouldNotParseMacAddress,
}

impl D30Config {
    pub fn load_toml(path: &PathBuf) -> Result<Self, D30Error> {
        let contents = fs::read_to_string(path).context(CouldNotReadFileSnafu)?;
        Ok(toml::from_str(contents.as_str()).context(CouldNotParseSnafu)?)
    }

    pub fn read_d30_config() -> Result<Self, D30Error> {
        let phomemo_lib_path = xdg::BaseDirectories::with_prefix("phomemo-library")
            .context(CouldNotGetXDGPathSnafu)?;
        let config_path = phomemo_lib_path
            .place_config_file("phomemo-config.toml")
            .context(CouldNotPlaceConfigFileSnafu)?;
        let toml = D30Config::load_toml(&config_path);
        if let Err(e) = &toml {
            warn!("Failed to parse config file: {:#?}", e);
        }
        toml
    }

    pub fn resolve_addr(&self, printer_addr: &String) -> Result<MacAddr6, D30Error> {
        match printer_addr.parse::<MacAddr6>() {
            Ok(mac_addr) => Ok(mac_addr),

            Err(e) => {
                trace!("Device specification `{}` is not a MAC Address. Assuming it's a hostname, and attempting resolution.", printer_addr);
                trace!("Inferred because: {}", e);
                let mac = self.resolution.get(printer_addr).context(
                    CouldNotParseOrLookupMacAddressSnafu {
                        device: printer_addr,
                    },
                )?;
                Ok(*mac)
            }
        }
    }

    pub fn resolve_default(&self) -> Result<MacAddr6, D30Error> {
        self.resolve_addr(self.default_device.as_ref().context(NoDefaultDeviceSnafu)?)
    }
}
