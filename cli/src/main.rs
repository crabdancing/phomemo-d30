// TODO: Figure out what's required for batch printing (e.g.,
// can I just send the precursor bytes once, and then send multiple packed images?
// TODO: Figure out how to handle non-precut labels
// TODO: Figure out how to handle 'fruit' labels
// TODO: Implement templates with fixed font sizes and positions
// TODO: Implement preview feature
// TODO: Implement 'arbitrary image' feature

use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    ops::DerefMut,
    process::Stdio,
    sync::Arc,
};

use advmac::MacAddr6;
use bluetooth_serial_port_async::BtAddr;
use clap::{Parser, Subcommand};
use d30::PrinterAddr;
use image::DynamicImage;
use log::debug;
use serde::{Deserialize, Serialize};
use snafu::{whatever, ResultExt, Snafu, Whatever};
use tokio::sync::Mutex;

#[derive(Debug, Parser)]
#[command(name = "d30")]
#[command(about = "A userspace Phomemo D30 controller.")]
/// `Arguments` stores the command line arguments passed in from the user or script
struct Arguments {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(short_flag = 't')]
    PrintText(ArgsPrintText),
}

#[derive(clap::Args, Debug)]
struct ArgsPrintText {
    #[arg(long)]
    dry_run: bool,
    #[arg(short, long)]
    device: Option<d30::PrinterAddr>,
    text: String,
    #[arg(short, long)]
    #[arg(default_value = "40")]
    scale: f32,
    #[arg(short, long)]
    preview: bool,
}

// ---------------------
// End CLI Processing

#[derive(Serialize, Deserialize)]
struct State {
    dry_run: Option<bool>,
    preview: Option<bool>,
    preview_cmd: Option<Vec<OsString>>,
    d30_config: Option<d30::D30Config>,
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

impl State {
    // fn new(args: &Arguments) -> Self {
    //     Self {
    //         dry_run: args.dry_run,
    //         d30_config: None,
    //     }
    // }
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
    let args = preview_cmd.unwrap_or(vec!["gio".into(), "open".into(), path]);
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

fn get_addr(
    state: &mut State,
    user_maybe_addr: Option<d30::PrinterAddr>,
) -> Result<MacAddr6, Whatever> {
    let addr: MacAddr6;
    match (user_maybe_addr, d30::D30Config::read_d30_config()) {
        // The case that the user has specified an address, and we have a config loaded
        // We must use config to attempt to resolve the address
        (Some(user_specified_addr), Ok(config)) => {
            let resolved_addr = config.resolve_addr(&user_specified_addr)?;
            addr = resolved_addr;
            state.d30_config = Some(config);
        }
        // The case that the user has specified an address, but we do NOT have a config
        // We must hope that the user gave us a fully quallified address & not a hostname
        (Some(user_specified_addr), Err(_)) => match user_specified_addr {
            PrinterAddr::MacAddr(user_addr) => {
                addr = user_addr;
            }
            PrinterAddr::PrinterName(name) => {
                whatever!(
                        "Cannot resolve \"{}\" because config file could not be retrieved.\n\
                        \tIf \"{}\" is meant to be an address rather than a device name, you should check your formatting,\n\
                        \tas it does not look like a valid MAC address.",
                        name, name
                    );
            }
        },
        // No address on CLI, but there IS a config!
        // Try to resolve from config
        (None, Ok(config)) => {
            addr = config
                .resolve_default()
                .with_whatever_context(|_| "Could not resolve default MAC address")?;
        }

        (None, Err(_)) => {
            whatever!("You did not correctly specify an address on command line or config file.")
        }
    }
    Ok(addr)
}

fn cmd_print(state: &mut State, args: &ArgsPrintText) -> Result<(), Whatever> {
    let dry_run = state.dry_run.unwrap_or(false) || args.dry_run;
    let show_preview = state.preview.unwrap_or(false) || args.preview;
    let addr = get_addr(state, args.device.clone())?;
    debug!("Generating image {} with scale {}", &args.text, &args.scale);
    let image = d30::generate_image(&args.text, args.scale)
        .with_whatever_context(|_| "Failed to generate image")?;
    let mut preview_image = image.rotate90();
    preview_image.invert();
    if show_preview {
        cmd_show_preview(state.preview_cmd.clone(), preview_image)?;
        let should_accept = inquire::Confirm::new("Displaying preview. Accept this print?")
            .with_default(false)
            .prompt_skippable()
            .with_whatever_context(|_| "Failed to ask user whether to accept")?
            .unwrap_or(false);
        if !should_accept {
            println!("Goodbye UwU");
            return Ok(());
        }
    }
    let mut socket =
        bluetooth_serial_port_async::BtSocket::new(bluetooth_serial_port_async::BtProtocol::RFCOMM)
            .with_whatever_context(|_| "Failed to open socket")?;

    if !dry_run {
        socket
            .connect(BtAddr(addr.to_array()))
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
    Ok(())
}

#[snafu::report]
#[tokio::main]
async fn main() -> Result<(), Whatever> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = Arguments::parse();
    debug!("Args: {:#?}", &args);
    let mut app = &args;
    match State::load_config() {
        Ok(mut state) => match &args.command {
            Commands::PrintText(args) => cmd_print(&mut state, &args)
                .with_whatever_context(|_| "Could not complete print command")?,
        },
        Err(_) => todo!(),
    }

    Ok(())
}
