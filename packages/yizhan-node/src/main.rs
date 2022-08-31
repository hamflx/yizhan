use std::time::Duration;

use clap::{Parser, Subcommand};
use client::YiZhanClient;
use error::YiZhanResult;
use log::info;
use network::YiZhanNetwork;
use random_names::RandomName;
use server::YiZhanServer;
use tcp::TcpServe;
use terminal::Terminal;
use tokio::time::sleep;
use yizhan_bootstrap::{
    install_bootstrap, install_program, is_running_process_installed, spawn_program,
};

mod client;
mod commands;
mod connection;
mod console;
mod error;
mod network;
mod serve;
mod server;
mod tcp;
mod terminal;

const YIZHAN_VERSION: &str = env!("CARGO_PKG_VERSION");
const IS_AUTO_INSTALL_ENABLED: bool = false;

#[tokio::main]
async fn main() -> YiZhanResult<()> {
    simple_logger::init().unwrap();

    info!("YiZhan v{}", YIZHAN_VERSION);

    if IS_AUTO_INSTALL_ENABLED {
        install();
        sleep(Duration::from_secs(1)).await;
    }

    let args = YiZhanArgs::parse();
    let name = if let Some(name) = args.name {
        name
    } else {
        RandomName::new().name
    };

    if args.command == Some(Action::Client) {
        info!("Running at client mode");

        let client = YiZhanClient::new().await?;
        let mut network = YiZhanNetwork::new(client, name);
        network.add_console(Box::new(Terminal::new())).await;
        network.run().await?;
    } else {
        info!("Running at server mode");
        let server = YiZhanServer::new(TcpServe::new().await?);
        let network = YiZhanNetwork::new(server, name);
        network.run().await?;
    }

    Ok(())
}

fn install() -> InstallResult {
    match is_running_process_installed(YIZHAN_VERSION) {
        Ok(false) | Err(_) => {
            let _ = install_bootstrap();
            let _ = install_program(YIZHAN_VERSION);
            let _ = spawn_program();
            print!("Run installed process ...");
            InstallResult::Installed
        }
        Ok(true) => InstallResult::RunningProcessInstalled,
    }
}

#[derive(PartialEq, Eq)]
enum InstallResult {
    RunningProcessInstalled,
    Installed,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct YiZhanArgs {
    #[clap(subcommand)]
    command: Option<Action>,

    #[clap(long, short, value_parser)]
    name: Option<String>,
}

#[derive(Subcommand, PartialEq, Eq, Debug)]
enum Action {
    Server,
    Client,
}
