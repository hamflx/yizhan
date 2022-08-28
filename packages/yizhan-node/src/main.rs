use clap::{Parser, Subcommand};
use client::YiZhanClient;
use error::YiZhanResult;
use log::info;
use network::YiZhanNetwork;
use server::YiZhanServer;
use tcp::TcpServe;
use terminal::Terminal;
use yizhan_bootstrap::{
    install_bootstrap, install_program, is_running_process_installed, spawn_program,
};

mod client;
mod connection;
mod console;
mod error;
mod network;
mod serve;
mod server;
mod tcp;
mod terminal;

const YIZHAN_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> YiZhanResult<()> {
    simple_logger::init().unwrap();

    if install() == InstallResult::Installed {
        return Ok(());
    }

    let args = YiZhanArgs::parse();

    if args.command == Action::Server {
        info!("Running at server mode");
        let server = YiZhanServer::new(TcpServe::new());
        let network = YiZhanNetwork::new(server);
        network.run().await?;
    } else {
        info!("Running at client mode");

        let client = YiZhanClient::new().await?;
        let mut network = YiZhanNetwork::new(client);
        network.add_console(Box::new(Terminal::new())).await;
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
    command: Action,
}

#[derive(Subcommand, PartialEq, Eq, Debug)]
enum Action {
    Server,
    Client,
}
