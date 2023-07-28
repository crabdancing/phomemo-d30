// TODO: load from config file -- store default font size & machine name -> mac addr mappings
// TODO: Encapsulate basic mechanisms for initializing connection and sending images
// TODO: Figure out what's required for batch printing (e.g.,
// can I just send the precursor bytes once, and then send multiple packed images?
use std::{io::Write, sync::Arc};

use clap::{Parser, Subcommand};
use d30::MacAddr;
use log::{debug, warn};
use snafu::{OptionExt, ResultExt, Whatever};
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
}

struct App {
    dry_run: bool,
    addr: Option<MacAddr>,
    d30_config: Option<d30::D30Config>,
}

impl App {
    fn new(args: &Cli) -> Self {
        Self {
            dry_run: args.dry_run,
            addr: None,
            d30_config: None,
        }
    }

    fn cmd_print(&mut self, args: &ArgsPrintText) -> Result<(), Whatever> {
        match self.d30_config {
            Some(_) => {}
            None => match d30::D30Config::read_d30_config() {
                Ok(d30_config) => self.d30_config = Some(d30_config),
                Err(e) => warn!("{:#?}", e),
            },
        }
        // if let Some(addr) = &args.addr {
        //     self.addr = Some(addr.clone());
        // } else {
        // }
        debug!("Generating image {} with scale {}", &args.text, &args.scale);
        let image = d30::generate_image(&args.text, args.scale)
            .with_whatever_context(|_| "Failed to generate image")?;
        // let addr = BtAddr([164, 7, 51, 76, 23, 54]);
        let mut socket = bluetooth_serial_port_async::BtSocket::new(
            bluetooth_serial_port_async::BtProtocol::RFCOMM,
        )
        .with_whatever_context(|_| "Failed to open socket")?;

        let addr = self.addr.clone().with_whatever_context(|| {
            "No address set. Set address via config file or `--addr` flag."
        })?;

        if !self.dry_run {
            socket
                .connect(addr.into())
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
