use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::command::{UserCommand, UserCommandResponse};

pub const WELCOME_MESSAGE: &str = "Welcome to YiZhan!";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum Message {
    Echo(String),
    CommandRequest(Option<String>, String, UserCommand),
    CommandResponse(Option<String>, String, UserCommandResponse),
}
