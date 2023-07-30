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
    sync::Arc,
};

use advmac::MacAddr6;
use bluetooth_serial_port_async::BtAddr;
use clap::{arg, Args, CommandFactory, Parser, Subcommand, ValueEnum};
use d30::PrinterAddr;
use image::DynamicImage;
use log::debug;
use merge::Merge;
use rusttype::Scale;
use serde::Deserialize;
use serde::Serialize;
use snafu::{prelude::*, whatever, ResultExt, Whatever};
use tokio::sync::Mutex;

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
    PrintText(ArgsPrintText),
    #[clap(short_flag = 'i')]
    PrintImage,
}

#[derive(Args, Debug, Serialize, Deserialize, Clone, Merge)]
struct ArgsPrintText {
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
impl ArgsPrintText {
    fn cmd_print_text(&self, app: &App) -> Result<(), Whatever> {
        match &self {
            ArgsPrintText {
                device: Some(device),
                text,
                scale: Some(scale),
                show_preview,
                preview_cmd,
            } => {
                let dry_run = app.dry_run.unwrap();
                let image = d30::generate_image_simple(&text, Scale::uniform(scale.clone()))
                    .with_whatever_context(|_| "Failed to generate image")?;
                let mut preview_image = image.rotate90();
                preview_image.invert();

                if show_preview.unwrap_or(false) {
                    cmd_show_preview(preview_cmd.clone(), preview_image)?;
                    if !inquire::Confirm::new("Proceed with print?")
                        .with_default(false)
                        .prompt()
                        .with_whatever_context(|_| "Could not prompt user")?
                    {
                        // Return early
                        return Ok(());
                    }
                }

                let mut socket = bluetooth_serial_port_async::BtSocket::new(
                    bluetooth_serial_port_async::BtProtocol::RFCOMM,
                )
                .with_whatever_context(|_| "Failed to open socket")?;

                let device = &app
                    .d30_config
                    .as_ref()
                    .unwrap()
                    .resolve_addr(device)
                    .unwrap();
                if !dry_run {
                    socket
                        .connect(BtAddr(device.to_array()))
                        .with_whatever_context(|_| "Failed to connect")?;
                }
                debug!("Init connection");
                if !dry_run {
                    socket
                        .write(d30::INIT_BASE_FLAT)
                        .with_whatever_context(|_| "Failed to send magic init bytes")?;
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
                        .with_whatever_context(|_| "Failed to write to socket")?;
                }
                debug!("Flush socket");
                if !dry_run {
                    socket
                        .flush()
                        .with_whatever_context(|_| "Failed to flush socket")?;
                }
            }

            _ => {
                whatever!(
                    "Data is left unspecified in config file or on command line. See: {:#?}",
                    &self
                )
            }
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

    //     let mut socket = bluetooth_serial_port_async::BtSocket::new(
    //         bluetooth_serial_port_async::BtProtocol::RFCOMM,
    //     )
    //     .with_whatever_context(|_| "Failed to open socket")?;

    //     if !self.dry_run {
    //         socket
    //             .connect(BtAddr(addr.to_array()))
    //             .with_whatever_context(|_| "Failed to connect")?;
    //     }
    //     debug!("Init connection");
    //     if !self.dry_run {
    //         socket
    //             .write(d30::INIT_BASE_FLAT)
    //             .with_whatever_context(|_| "Failed to send magic init bytes")?;
    //     }
    //     let mut output = d30::IMG_PRECURSOR.to_vec();
    //     debug!("Extend output");
    //     if !self.dry_run {
    //         output.extend(d30::pack_image(&image));
    //     }
    //     debug!("Write output to socket");
    //     if !self.dry_run {
    //         socket
    //             .write(output.as_slice())
    //             .with_whatever_context(|_| "Failed to write to socket")?;
    //     }
    //     debug!("Flush socket");
    //     if !self.dry_run {
    //         socket
    //             .flush()
    //             .with_whatever_context(|_| "Failed to flush socket")?;
    //     }
    //     Ok(())
    // }
}

#[snafu::report]
#[tokio::main]
async fn main() -> Result<(), Whatever> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    // let app = clap::Command::new("test").args(&Conf::clap_args());

    let mut base = App::parse();

    let file_layer =
        App::load_config().with_whatever_context(|_| "Could not load config from file")?;

    base.merge(file_layer);

    match base.commands.clone() {
        Some(Commands::PrintText(args)) => args.cmd_print_text(&base)?,
        Some(Commands::PrintImage) => todo!(),
        None => {
            whatever!("You must specify a command. Pass `--help` flag to see available commands");
        }
    }
    // let conf = toml::to_string_pretty(&base).unwrap();

    // let args = Cli::command().get_matches();
    // println!("{}", &a);
    // debug!("Args: {:#?}", &args);
    // let app = Arc::new(Mutex::new(App::new(&args)));

    // match &args.command {
    //     Commands::PrintText(args) => app
    //         .lock()
    //         .await
    //         .cmd_print(&args)
    //         .with_whatever_context(|_| "Could not complete print command")?,
    // }

    Ok(())
}
