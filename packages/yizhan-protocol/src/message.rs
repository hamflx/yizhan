use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::command::{Command, CommandResponse};

pub const WELCOME_MESSAGE: &str = "Welcome to YiZhan!";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum Message {
    Echo(String),
    Command(Option<String>, String, Command),
    CommandResponse(Option<String>, String, CommandResponse),
}
