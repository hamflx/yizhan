use anyhow::Result;

use server::YiZhanServer;
use tcp::TcpServe;
use yizhan_bootstrap::{
    install_bootstrap, install_program, is_running_process_installed, spawn_program,
};

mod error;
mod serve;
mod server;
mod tcp;

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

    let server = YiZhanServer::new(TcpServe {});
    server.run().await?;

    Ok(())
}
