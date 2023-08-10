use std::io;
use std::{fmt::Display, fs, path::PathBuf, str::FromStr};

use advmac::MacAddr6;
use image::{DynamicImage, ImageBuffer, Rgb};
use log::{trace, warn};
use rusttype::{Font, Scale};

use dimensions::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use snafu::{Error, OptionExt, ResultExt, Snafu, Whatever};
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

pub fn generate_image(text: &str, font_scale: f32) -> Result<DynamicImage, Whatever> {
    let dim = Dimensions::new(320, 96);
    trace!("{:#?}", &dim);
    let font = Vec::from(include_bytes!("DejaVuSans.ttf") as &[u8]);
    let font = Font::try_from_vec(font).with_whatever_context(|| "Failed to parse font data")?;
    let scale = Scale::uniform(font_scale);

    let actual_size: Dimensions = imageproc::drawing::text_size(scale, &font, &text).into();

    let txt_pos = (actual_size - dim) / -2.;

    let mut canvas: ImageBuffer<Rgb<u8>, _> =
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrinterAddr {
    MacAddr(MacAddr6),
    PrinterName(String),
}

impl PrinterAddr {
    pub fn to_string(&self) -> String {
        match self {
            PrinterAddr::MacAddr(a) => a.to_string(),
            PrinterAddr::PrinterName(s) => s.into(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match MacAddr6::from_str(s) {
            Ok(addr) => PrinterAddr::MacAddr(addr),
            Err(_) => PrinterAddr::PrinterName(s.to_owned()),
        }
    }
}

impl Into<String> for PrinterAddr {
    fn into(self) -> String {
        match self {
            PrinterAddr::MacAddr(addr) => addr.to_string(),
            PrinterAddr::PrinterName(name) => name,
        }
    }
}

impl Into<String> for &PrinterAddr {
    fn into(self) -> String {
        match &self {
            PrinterAddr::MacAddr(addr) => addr.to_string(),
            PrinterAddr::PrinterName(name) => name.clone(),
        }
    }
}

impl From<MacAddr6> for PrinterAddr {
    fn from(value: MacAddr6) -> Self {
        PrinterAddr::MacAddr(value)
    }
}

impl FromStr for PrinterAddr {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match MacAddr6::from_str(s) {
            Ok(v) => Ok(PrinterAddr::MacAddr(v)),
            Err(_) => Ok(PrinterAddr::PrinterName(s.to_string())),
        }
    }
}

impl Display for PrinterAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.into();
        write!(f, "{}", s)?;
        Ok(())
    }
}
mod printer_addr_serde {
    use super::PrinterAddr;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &PrinterAddr, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PrinterAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PrinterAddr::from_string(&s))
    }
}

// D30Config

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct D30Config {
    #[serde(with = "printer_addr_serde")]
    pub default: PrinterAddr,
    pub resolution: IndexMap<String, MacAddr6>,
}

#[derive(Debug, Snafu)]
pub enum LoadTomlError {
    #[snafu(display("Failed to read in automatically detected D30 library configuration path"))]
    CouldNotReadFile { source: io::Error },
    #[snafu(display("Failed to serialize TOML D30 config"))]
    CouldNotParse { source: toml::de::Error },
}

#[derive(Debug, Snafu)]
pub enum ReadD30ConfigError {
    #[snafu(display("Could not get XDG path"))]
    CouldNotGetXDGPath { source: xdg::BaseDirectoriesError },
    #[snafu(display("Could not place config file"))]
    CouldNotPlaceConfigFile { source: io::Error },
    #[snafu(display("Could not load TOML"))]
    CouldNotLoadToml { source: LoadTomlError },
}
type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

impl D30Config {
    pub fn load_toml(path: &PathBuf) -> Result<Self, LoadTomlError> {
        let contents = fs::read_to_string(path).context(CouldNotReadFileSnafu)?;
        Ok(toml::from_str(contents.as_str()).context(CouldNotParseSnafu)?)
    }

    pub fn read_d30_config() -> Result<Self, ReadD30ConfigError> {
        let phomemo_lib_path = xdg::BaseDirectories::with_prefix("phomemo-library")
            .context(CouldNotGetXDGPathSnafu)?;
        let config_path = phomemo_lib_path
            .place_config_file("phomemo-config.toml")
            .context(CouldNotPlaceConfigFileSnafu)?;
        let toml = D30Config::load_toml(&config_path);
        if let Err(e) = &toml {
            warn!("Failed to parse config file: {:#?}", e);
        }
        Ok(toml.context(CouldNotLoadTomlSnafu)?)
    }

    pub fn resolve_addr(&self, printer_addr: &PrinterAddr) -> Result<MacAddr6, Whatever> {
        match printer_addr {
            PrinterAddr::MacAddr(addr) => Ok(addr.clone()),
            PrinterAddr::PrinterName(name) => {
                let mac = self.resolution.get(name).with_whatever_context(|| {
                    format!(
                        "Could not parse MAC address, or find in hostname table: {}",
                        name
                    )
                })?;
                Ok(mac.clone())
            }
        }
    }

    pub fn resolve_default(&self) -> Result<MacAddr6, Whatever> {
        self.resolve_addr(&self.default)
    }
}
