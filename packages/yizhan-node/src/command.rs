#[derive(Debug)]
pub(crate) enum Command {
    Echo(String),
}

unsafe impl Send for Command {}

unsafe impl Sync for Command {}
