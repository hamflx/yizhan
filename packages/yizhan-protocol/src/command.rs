use std::str::FromStr;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum Command {
    Echo(String),
    Update,
}

impl FromStr for Command {
    type Err = ParseCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "update" => Command::Update,
            _ => return Err(ParseCommandError::UnrecognizedCommand),
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseCommandError {
    #[error("data store disconnected")]
    UnrecognizedCommand,
}

unsafe impl Send for Command {}

unsafe impl Sync for Command {}
