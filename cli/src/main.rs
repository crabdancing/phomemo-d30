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

use std::{ffi::OsString, io::Write, process::Stdio, sync::Arc};

use advmac::MacAddr6;
use bluetooth_serial_port_async::BtAddr;
use clap::{Parser, Subcommand};
use d30::{PrinterAddr, ReadD30ConfigError};
use image::DynamicImage;
use log::debug;
use rusttype::Scale;
use snafu::{whatever, ResultExt, Whatever};
use tokio::sync::Mutex;

#[derive(Debug, Parser)]
#[command(name = "d30")]
#[command(about = "A userspace Phomemo D30 controller.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long)]
    dry_run: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(short_flag = 't')]
    PrintText(ArgsPrintText),
}

#[derive(clap::Args, Debug)]
struct ArgsPrintText {
    #[arg(short, long)]
    addr: Option<d30::PrinterAddr>,
    text: String,
    #[arg(short, long)]
    #[arg(default_value = "40")]
    scale: f32,
    #[arg(short, long)]
    show_preview: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct AppSettings {
    show_preview: Option<bool>,
}

impl AppSettings {
    fn load_config() -> Result<Self, ReadD30ConfigError> {
        let phomemo_lib_path = xdg::BaseDirectories::with_prefix("phomemo-library")
            .context(CouldNotGetXDGPathSnafu)?;
        let config_path = phomemo_lib_path
            .place_config_file("phomemo-cli-config.toml")
            .context(CouldNotPlaceConfigFileSnafu)?;
        let contents = fs::read_to_string(config_path).context(CouldNotReadFileSnafu)?;
        Ok(toml::from_str(contents.as_str()).context(CouldNotParseSnafu)?)
    }

    fn get_show_preview(&self) -> bool {
        self.show_preview.unwrap_or(false)
    }
}

struct App {
    dry_run: bool,
    d30_config: Option<d30::D30Config>,
    app_settings: Result<AppSettings, ReadD30ConfigError>,
    preview_cmd: Option<Vec<OsString>>,
    show_preview: bool,
}

impl App {
    fn new(args: &Cli) -> Self {
        let app_settings = AppSettings::load_config();
        Self {
            dry_run: args.dry_run,
            d30_config: None,
            app_settings,
            preview_cmd: None,
            show_preview: app_settings.get_show_preview(),
        }
    }

    fn compute_show_preview(&mut self) -> bool {
        let mut show_preview: bool;
        match (args.show_preview, app_settings) {
            (Some(true), _) => {
                show_preview = true;
            }
            (Some(false), _) => {
                show_preview = false;
            }
            (
                None,
                Ok(AppSettings {
                    show_preview: Some(true),
                }),
            ) => {
                show_preview = true;
            }

            (None, _) => {
                show_preview = false;
            }
        }
        show_preview
    }

    fn get_addr(
        &mut self,
        user_maybe_addr: Option<d30::PrinterAddr>,
    ) -> Result<MacAddr6, Whatever> {
        let addr: MacAddr6;
        match (user_maybe_addr, d30::D30Config::read_d30_config()) {
            // The case that the user has specified an address, and we have a config loaded
            // We must use config to attempt to resolve the address
            (Some(user_specified_addr), Ok(config)) => {
                let resolved_addr = config.resolve_addr(&user_specified_addr)?;
                addr = resolved_addr;
                self.d30_config = Some(config);
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
                        \tIf it is meant to be an address rather than a device name, you should check your formatting,\n\
                        \tas it does not look like a valid MAC address.",
                        name
                    );
                }
            },
            // No address on CLI, but there IS a config!
            // Try to resolve from config
            (None, Ok(config)) => match &config {
                d30::D30Config {
                    default: PrinterAddr::MacAddr(default_addr),
                    resolution: _,
                } => {
                    addr = *default_addr;
                }
                d30::D30Config {
                    default: PrinterAddr::PrinterName(_),
                    resolution: _,
                } => {
                    addr = config
                        .resolve_default()
                        .with_whatever_context(|_| "Could not resolve default MAC address")?;
                }
            },
            // No address specified on CLI, and errored when config load was attempted
            // Just print errors and exit
            (None, Err(_)) => {
                whatever!(
                    "You did not correctly specify an address on command line or config file."
                )
            }
        }
        Ok(addr)
    }

    fn cmd_print(&mut self, args: &ArgsPrintText) -> Result<(), Whatever> {
        let addr = self.get_addr(args.addr.clone())?;
        debug!("Generating image {} with scale {}", &args.text, &args.scale);
        let image = d30::generate_image_simple(&args.text, Scale::uniform(args.scale))
            .with_whatever_context(|_| "Failed to generate image")?;

        self.compute_show_preview();

        if self.show_preview {
            let mut preview_image = image.rotate90();
            preview_image.invert();
            self.show_preview(preview_image)?;
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

        if !self.dry_run {
            socket
                .connect(BtAddr(addr.to_array()))
                .with_whatever_context(|_| "Failed to connect")?;
        }
        debug!("Init connection");
        if !self.dry_run {
            socket
                .write(d30::INIT_BASE_FLAT)
                .with_whatever_context(|_| "Failed to send magic init bytes")?;
        }
        let mut output = d30::IMG_PRECURSOR.to_vec();
        debug!("Extend output");
        if !self.dry_run {
            output.extend(d30::pack_image(&image));
        }
        debug!("Write output to socket");
        if !self.dry_run {
            socket
                .write(output.as_slice())
                .with_whatever_context(|_| "Failed to write to socket")?;
        }
        debug!("Flush socket");
        if !self.dry_run {
            socket
                .flush()
                .with_whatever_context(|_| "Failed to flush socket")?;
        }
        Ok(())
    }
    fn show_preview(&self, preview_image: DynamicImage) -> Result<(), Whatever> {
        let preview_image_file = temp_file::TempFile::new()
            .with_whatever_context(|_| "Failed to make temporary file")?;
        let path = preview_image_file
            .path()
            .with_extension("jpg")
            .into_os_string();
        preview_image
            .save(&path)
            .with_whatever_context(|_| "Failed to write to temporary file")?;
        let args = self
            .preview_cmd
            .clone()
            .unwrap_or(vec!["gio".into(), "open".into(), path]);
        run_with_args(args)?;
        Ok(())
    }
}

fn run_with_args(args: Vec<OsString>) -> Result<std::process::Child, Whatever> {
    match args.as_slice() {
        [cmd, args @ ..] => std::process::Command::new(cmd)
            .args(args)
            .stderr(Stdio::piped())
            .spawn()
            .with_whatever_context(|_| format!("Failed to execute child process: {:?}", cmd)),

        [] => {
            whatever!("No program specified");
        }
    }
}

#[snafu::report]
#[tokio::main]
async fn main() -> Result<(), Whatever> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = Cli::parse();
    debug!("Args: {:#?}", &args);
    let app = Arc::new(Mutex::new(App::new(&args)));

    match &args.command {
        Commands::PrintText(args) => app
            .lock()
            .await
            .cmd_print(&args)
            .with_whatever_context(|_| "Could not complete print command")?,
    }

    Ok(())
}
