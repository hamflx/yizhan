use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommand {
    Update,
    Run(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommandResponse {
    Run(CommandRunResult),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum CommandRunResult {
    Success(String),
    Failed(String),
}

unsafe impl Send for UserCommand {}

unsafe impl Sync for UserCommand {}
