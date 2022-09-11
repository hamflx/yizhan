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
    Ls,
    PluginCommand(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommandResponse {
    Update,
    Run(String),
    Ls(Vec<ListedNodeInfo>),
    PluginCommand(String),
    PluginBinaryCommand(String, String, Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub struct NodeInfo {
    pub id: String,
    pub mac: String,
    pub version: VersionInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub struct ListedNodeInfo {
    pub id: String,
    pub ip: String,
    pub mac: String,
    pub version: VersionInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommandResult {
    Ok(UserCommandResponse),
    Err(String),
}

unsafe impl Send for UserCommand {}

unsafe impl Sync for UserCommand {}
