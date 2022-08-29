use std::str::FromStr;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum Command {
    Echo(String),
    Update,
    Run(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum CommandResponse {
    Run(String),
}

impl FromStr for Command {
    type Err = ParseCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed_line = s.trim();
        let (command, args) = match trimmed_line.split_once(' ') {
            Some((c, a)) => (c.trim(), Some(a.trim())),
            _ => (trimmed_line, None),
        };
        Ok(match (command, args) {
            ("update", _) => Command::Update,
            ("run", Some(args)) => Command::Run(args.to_string()),
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
