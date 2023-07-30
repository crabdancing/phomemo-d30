// TODO: option to change preview program in config file
// TODO: option to preview by default, or not preview by default, in config file
// TODO: Figure out what's required for batch printing (e.g.,
// can I just send the precursor bytes once, and then send multiple packed images?
// TODO: Figure out how to handle non-precut labels
// TODO: Figure out how to handle 'fruit' labels
// TODO: Implement templates with fixed font sizes and positions
// TODO: toggle preview
// TODO: toggle COMPILING preview into program
// TODO: have window close
// TODO: Implement 'arbitrary image' feature

use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    process::Stdio,
};

use advmac::MacAddr6;
use bluetooth_serial_port_async::BtAddr;
use clap::{arg, Args, Parser, Subcommand};
use image::DynamicImage;
use log::debug;
use merge::Merge;
use rusttype::Scale;
use serde::Deserialize;
use serde::Serialize;
use snafu::{prelude::*, whatever, OptionExt, ResultExt, Whatever};

#[derive(Parser, Debug, Serialize, Deserialize, Clone, Merge)]
// #[command(name = "d30")]
// #[command(about = "A userspace Phomemo D30 controller.")]
struct App {
    #[clap(short, long)]
    dry_run: Option<bool>,
    // #[clap(short, long)]
    #[command(subcommand)]
    #[merge(skip)]
    commands: Option<Commands>,
    #[clap(skip)]
    d30_config: Option<d30::D30Config>,
}

#[derive(Subcommand, Debug, Serialize, Deserialize, Clone)]
enum Commands {
    #[clap(short_flag = 't')]
    PrintText(CmdPrintText),
    #[clap(short_flag = 'i')]
    PrintImage,
}

#[derive(Args, Debug, Serialize, Deserialize, Clone, Merge)]
struct CmdPrintText {
    #[arg(short, long)]
    device: Option<d30::PrinterAddr>,
    #[merge(skip)]
    text: String,
    #[arg(short, long)]
    #[arg(default_value = "40")]
    scale: Option<f32>,
    #[arg(long, short = 'p')]
    show_preview: Option<bool>,
    #[clap(short = 'c', long)]
    preview_cmd: Option<Vec<OsString>>,
}

#[derive(Debug, Snafu)]
pub enum ReadD30CliConfigError {
    #[snafu(display("Could not get XDG path"))]
    CouldNotGetXDGPath { source: xdg::BaseDirectoriesError },
    #[snafu(display("Could not place config file"))]
    CouldNotPlaceConfigFile { source: io::Error },
    #[snafu(display("Failed to read in automatically detected D30 CLI configuration path"))]
    CouldNotReadFile { source: io::Error },
    #[snafu(display("Failed to serialize TOML D30 config"))]
    CouldNotParse { source: toml::de::Error },
}

fn cmd_show_preview(
    preview_cmd: Option<Vec<OsString>>,
    preview_image: DynamicImage,
) -> Result<(), Whatever> {
    let preview_image_file =
        temp_file::TempFile::new().with_whatever_context(|_| "Failed to make temporary file")?;
    let path = preview_image_file
        .path()
        .with_extension("jpg")
        .into_os_string();
    debug!("{:?}", &path);
    preview_image
        .save(&path)
        .with_whatever_context(|_| "Failed to write to temporary file")?;
    let args = preview_cmd.unwrap_or(vec!["oculante".into(), path]);
    run(args)?;
    Ok(())
}

fn run(args: Vec<OsString>) -> Result<std::process::Child, Whatever> {
    debug!("Running child process: {:?}", args);
    match args.as_slice() {
        [cmd, args @ ..] => std::process::Command::new(cmd)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_whatever_context(|_| format!("Failed to execute child process: {:?}", cmd)),

        [] => {
            whatever!("No program specified");
        }
    }
}

#[derive(Debug, Snafu)]
enum CmdPrintTextErrors {
    #[snafu(display("Device not specified. See `--help` for how to specify the device, or add it to the config file."))]
    DeviceNotSpecified,
    #[snafu(display("Failed to generate image"))]
    FailedToGenerateImage { source: Whatever },
    #[snafu(display("Could not show user a preview"))]
    FailedToShowPreview { source: Whatever },
    #[snafu(display("Could not prompt user"))]
    CouldNotPromptUser { source: inquire::InquireError },
    #[snafu(display("Failed to open socket"))]
    FailedToOpenSocket {
        source: bluetooth_serial_port_async::BtError,
    },
    #[snafu(display("Failed to connect to device"))]
    FailedToConnect {
        source: bluetooth_serial_port_async::BtError,
    },
    #[snafu(display("Failed to send magic init bytes"))]
    FailedToSendMagic { source: std::io::Error },
    #[snafu(display("Failed to write to socket"))]
    FailedToWriteToSocket { source: std::io::Error },
    #[snafu(display("Failed to flush"))]
    FailedToFlush { source: std::io::Error },
    #[snafu(display("Scale of font not specified"))]
    ScaleNotSpecified,
    #[snafu(display("Failed to resolve address. Check your Rust d30 config file"))]
    FailedToResolveAddr { source: Whatever },
    #[snafu(whatever)]
    Whatever { message: String },
}

impl CmdPrintText {
    fn cmd_print_text(&self, app: &App) -> Result<(), CmdPrintTextErrors> {
        let scale = self.scale.with_context(|| ScaleNotSpecifiedSnafu)?;
        let dry_run = app.dry_run.unwrap_or(false);
        let d30_config = app.d30_config.as_ref();

        let d30_config_default_device = match d30_config {
            Some(d30_config) => d30_config.default.clone(),
            None => None,
        };

        let device = match (self.device.clone(), d30_config_default_device) {
            (Some(device), _) => Ok(device),
            (_, Some(device)) => Ok(device),
            (_, _) => Err(CmdPrintTextErrors::DeviceNotSpecified),
        }?;

        let image = d30::generate_image_simple(&self.text, Scale::uniform(scale.clone()))
            .context(FailedToGenerateImageSnafu)?;
        let mut preview_image = image.rotate90();
        preview_image.invert();

        if self.show_preview.unwrap_or(false) {
            cmd_show_preview(self.preview_cmd.clone(), preview_image)
                .context(FailedToShowPreviewSnafu)?;
            if !inquire::Confirm::new("Proceed with print?")
                .with_default(false)
                .prompt()
                .context(CouldNotPromptUserSnafu)?
            {
                // Return early
                return Ok(());
            }
        }

        let mut socket = bluetooth_serial_port_async::BtSocket::new(
            bluetooth_serial_port_async::BtProtocol::RFCOMM,
        )
        .context(FailedToOpenSocketSnafu)?;

        // Resolve address, whether or not D30Config is available
        let device = match d30_config {
            Some(d30_config) => d30_config
                .resolve_addr(&device)
                .context(FailedToResolveAddrSnafu)?,
            None => match device {
                d30::PrinterAddr::MacAddr(mac_addr) => mac_addr,
                d30::PrinterAddr::PrinterName(_) => {
                    whatever!("Device identifier is not a valid MAC address, and there is no TOML lookup table");
                }
            },
        };

        if !dry_run {
            socket
                .connect(BtAddr(device.to_array()))
                .context(FailedToConnectSnafu)?;
        }
        debug!("Init connection");
        if !dry_run {
            socket
                .write(d30::INIT_BASE_FLAT)
                .context(FailedToSendMagicSnafu)?;
        }
        let mut output = d30::IMG_PRECURSOR.to_vec();
        debug!("Extend output");
        if !dry_run {
            output.extend(d30::pack_image(&image));
        }
        debug!("Write output to socket");
        if !dry_run {
            socket
                .write(output.as_slice())
                .context(FailedToWriteToSocketSnafu)?;
        }
        debug!("Flush socket");
        if !dry_run {
            socket.flush().context(FailedToFlushSnafu)?;
        }
        Ok(())
    }
}

impl App {
    fn load_config() -> Result<Self, ReadD30CliConfigError> {
        let phomemo_lib_path = xdg::BaseDirectories::with_prefix("phomemo-library")
            .context(CouldNotGetXDGPathSnafu)?;
        let config_path = phomemo_lib_path
            .place_config_file("phomemo-cli-config.toml")
            .context(CouldNotPlaceConfigFileSnafu)?;
        let contents = fs::read_to_string(config_path).context(CouldNotReadFileSnafu)?;
        Ok(toml::from_str(contents.as_str()).context(CouldNotParseSnafu)?)
    }
}

#[snafu::report]
#[tokio::main]
async fn main() -> Result<(), Whatever> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let mut base = App::parse();

    base.d30_config = Some(
        d30::D30Config::read_d30_config().with_whatever_context(|_| "Failed to read D30 config")?,
    );

    let file_layer =
        App::load_config().with_whatever_context(|_| "Could not load config from file")?;

    base.merge(file_layer);

    match base.commands.clone() {
        Some(Commands::PrintText(print_text)) => print_text
            .cmd_print_text(&base)
            .with_whatever_context(|_| "Failed to print text")?,
        Some(Commands::PrintImage) => todo!(),
        None => {
            whatever!("You must specify a command. Pass `--help` flag to see available commands");
        }
    }

    Ok(())
}
