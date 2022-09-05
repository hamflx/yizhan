#![cfg_attr(windows, windows_subsystem = "windows")]

use std::time::Duration;

use clap::{Parser, Subcommand};
use client::YiZhanClient;
use config::YiZhanNodeConfig;
use error::YiZhanResult;
use network::YiZhanNetwork;
use random_names::RandomName;
use server::YiZhanServer;
use tcp::TcpServe;
use terminal::local::LocalTerminal;
use tokio::time::sleep;
use tracing::{info, Level};
use yizhan_bootstrap::{
    install_bootstrap, install_program, is_running_process_installed, set_auto_start, spawn_program,
};
use yizhan_protocol::version::VersionInfo;

use crate::{console::Console, terminal::remote::RemoteTerminal};

mod client;
mod commands;
mod config;
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

    let default_mode = if is_running_process_installed(&version).unwrap_or_default() {
        Some(Action::Client)
    } else {
        None
    };
    let mode = args.command.or(default_mode);
    let predefined_config = include_str!("../../../yizhan.toml");
    let config: YiZhanNodeConfig = toml::from_str(predefined_config).unwrap();

    if mode == Some(Action::Server) {
        info!("Running at server mode");
        let server = YiZhanServer::new(TcpServe::new(&config.server).await?);
        let network = YiZhanNetwork::new(server, name, version, true, config);
        network.run().await?;
    } else if mode == Some(Action::Client) {
        info!("Running at client mode");

        let client = YiZhanClient::new()?;
        let mut network = YiZhanNetwork::new(client, name, version, false, config);
        let terminal: Box<dyn Console> = if args.terminal {
            Box::new(LocalTerminal::new())
        } else {
            Box::new(RemoteTerminal::new())
        };
        network.add_console(terminal).await;
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

    #[clap(long, short)]
    terminal: bool,
}

#[derive(Subcommand, PartialEq, Eq, Debug)]
enum Action {
    Server,
    Client,
}
