use yizhan_bootstrap::{release_bootstrap, release_program};

const BOOTSTRAP_PAYLOAD: &[u8] = include_bytes!("../../../target/debug/yizhan-node.exe");

fn main() -> anyhow::Result<()> {
    release_bootstrap()?;
    release_program(BOOTSTRAP_PAYLOAD)?;

    Ok(())
}
