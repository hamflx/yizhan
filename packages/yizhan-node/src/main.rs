use anyhow::Result;

use client::YiZhanClient;
use network::YiZhanNetwork;
use server::YiZhanServer;
use tcp::TcpServe;
use terminal::Terminal;
use yizhan_bootstrap::{
    install_bootstrap, install_program, is_running_process_installed, spawn_program,
};

mod client;
mod command;
mod console;
mod error;
mod network;
mod serve;
mod server;
mod tcp;
mod terminal;

#[tokio::main]
async fn main() -> Result<()> {
    match is_running_process_installed() {
        Ok(false) | Err(_) => {
            let _ = install_bootstrap();
            let _ = install_program();
            let _ = spawn_program();
            print!("Run installed process ...");
            return Ok(());
        }
        _ => {}
    }

    let client = YiZhanClient::new(Terminal::new());

    let network = YiZhanNetwork::new(YiZhanServer::new(TcpServe {}), client);
    network.run().await?;

    Ok(())
}
