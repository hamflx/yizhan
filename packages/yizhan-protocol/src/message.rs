use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const WELCOME_MESSAGE: &str = "Welcome to YiZhan!";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum Message {
    Echo(String),
}
