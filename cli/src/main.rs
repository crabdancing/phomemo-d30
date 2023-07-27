// TODO: load from config file -- store default font size & machine name -> mac addr mappings
// TODO: Encapsulate basic mechanisms for initializing connection and sending images
// TODO: Figure out what's required for batch printing (e.g.,
// can I just send the precursor bytes once, and then send multiple packed images?
use std::{io::Write, str::FromStr, sync::Arc};

use bluetooth_serial_port_async::BtAddr;
use clap::{Parser, Subcommand};
use log::debug;
use snafu::{ResultExt, Whatever};
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
    addr: MacAddr,
    text: String,
    #[arg(short, long)]
    #[arg(default_value = "40")]
    scale: f32,
}

#[derive(Clone, Debug)]
struct MacAddr([u8; 6]);

impl Into<String> for MacAddr {
    fn into(self) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
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

impl Into<BtAddr> for MacAddr {
    fn into(self) -> BtAddr {
        BtAddr(self.0)
    }
}

struct App {
    dry_run: bool,
}

impl App {
    fn new(args: &Cli) -> Self {
        Self {
            dry_run: args.dry_run,
        }
    }

    fn cmd_print(&mut self, args: &ArgsPrintText) -> Result<(), Whatever> {
        debug!("Generating image {} with scale {}", &args.text, &args.scale);
        let image = d30::generate_image(&args.text, args.scale)
            .with_whatever_context(|_| "Failed to generate image")?;
        // let addr = BtAddr([164, 7, 51, 76, 23, 54]);
        let mut socket = bluetooth_serial_port_async::BtSocket::new(
            bluetooth_serial_port_async::BtProtocol::RFCOMM,
        )
        .with_whatever_context(|_| "Failed to open socket")?;

        if !self.dry_run {
            socket
                .connect(args.addr.clone().into())
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
