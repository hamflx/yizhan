use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::version::VersionInfo;

type FileSha256 = String;
type Platform = String;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommand {
    Halt,
    Update(VersionInfo, Platform, FileSha256, Vec<u8>),
    Run(String, Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommandResponse {
    Update(CommandUpdateResult),
    Run(CommandRunResult),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum CommandRunResult {
    Success(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum CommandUpdateResult {
    Success,
    Failed(String),
}

unsafe impl Send for UserCommand {}

unsafe impl Sync for UserCommand {}
