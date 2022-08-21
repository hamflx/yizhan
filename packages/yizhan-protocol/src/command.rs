use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum Command {
    Echo(String),
}

unsafe impl Send for Command {}

unsafe impl Sync for Command {}
