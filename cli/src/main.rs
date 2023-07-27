use std::{io::Write, sync::Arc};

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
    text: String,
    #[arg(short, long)]
    #[arg(default_value = "40")]
    scale: f32,
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
        let image = d30::generate_image(&args.text, args.scale);
        let addr = BtAddr([164, 7, 51, 76, 23, 54]);
        let mut socket = bluetooth_serial_port_async::BtSocket::new(
            bluetooth_serial_port_async::BtProtocol::RFCOMM,
        )
        .unwrap();

        if !self.dry_run {
            socket
                .connect(addr)
                .with_whatever_context(|_| "Failed to connect")?;
        }
        debug!("Init connection");
        if !self.dry_run {
            d30::init_conn(&mut socket).with_whatever_context(|_| "Failed to init connection")?;
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
