use std::time::Duration;

use clap::{Parser, Subcommand};
use client::YiZhanClient;
use error::YiZhanResult;
use network::YiZhanNetwork;
use random_names::RandomName;
use server::YiZhanServer;
use tcp::TcpServe;
use terminal::Terminal;
use tokio::time::sleep;
use tracing::{info, Level};
use yizhan_bootstrap::{
    install_bootstrap, install_program, is_running_process_installed, set_auto_start, spawn_program,
};
use yizhan_protocol::version::VersionInfo;

mod client;
mod commands;
mod connection;
mod console;
mod context;
mod error;
mod message;
mod network;
mod serve;
mod server;
mod tcp;
mod terminal;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const IS_AUTO_INSTALL_ENABLED: bool = false;

#[tokio::main]
async fn main() -> YiZhanResult<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    let mut version: VersionInfo = CARGO_PKG_VERSION.try_into()?;
    version.set_build_no(env!("VERSION_BUILD_NO").parse()?);
    info!("YiZhan v{}", version.to_string());

    if IS_AUTO_INSTALL_ENABLED {
        install(&version);
        sleep(Duration::from_secs(1)).await;
    }

    let args = YiZhanArgs::parse();
    let name = if let Some(name) = args.name {
        name
    } else {
        RandomName::new().name
    };

    let default_mode = if is_running_process_installed(&version).unwrap_or_default() == true {
        Some(Action::Client)
    } else {
        None
    };
    let mode = args.command.or(default_mode);

    if mode == Some(Action::Server) {
        info!("Running at server mode");
        let server = YiZhanServer::new(TcpServe::new().await?);
        let network = YiZhanNetwork::new(server, name, version, true);
        network.run().await?;
    } else if mode == Some(Action::Client) {
        info!("Running at client mode");

        let client = YiZhanClient::new().await?;
        let mut network = YiZhanNetwork::new(client, name, version, false);
        network.add_console(Box::new(Terminal::new())).await;
        network.run().await?;
    } else {
        install(&version);
    }

    Ok(())
}

fn install(version: &VersionInfo) -> InstallResult {
    match is_running_process_installed(version) {
        Ok(false) | Err(_) => {
            let _ = install_bootstrap();
            let _ = install_program(version);
            let _ = spawn_program();
            let _ = set_auto_start();
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
