use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::command::{UserCommand, UserCommandResponse};

pub const WELCOME_MESSAGE: &str = "Welcome to YiZhan!";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum Message {
    Echo(String),
    CommandRequest {
        target: Option<String>,
        source: Option<String>,
        cmd_id: String,
        cmd: UserCommand,
    },
    CommandResponse(Option<String>, String, UserCommandResponse),
}
