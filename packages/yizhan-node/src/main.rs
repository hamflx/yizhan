use yizhan_bootstrap::{install_program, is_running_process_installed, spawn_program};

fn main() {
    match is_running_process_installed() {
        Ok(false) | Err(_) => {
            let _ = install_program();
            let _ = spawn_program();
            return;
        }
        _ => {}
    }
    println!("Hello, world!");
}
