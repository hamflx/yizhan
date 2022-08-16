use serde::{Deserialize, Serialize};

pub const WELCOME_MESSAGE: &str = "Welcome to YiZhan!";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Message {
    Echo(String),
}
