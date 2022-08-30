use std::str::FromStr;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommand {
    Update,
    Run(Option<String>, String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub enum UserCommandResponse {
    Run(String),
}

impl FromStr for UserCommand {
    type Err = ParseCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = split_command_args(s);
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        let args = args.as_slice();

        Ok(match args {
            &["update"] => UserCommand::Update,
            &["run", cmd] => UserCommand::Run(None, cmd.to_string()),
            &["run", node_id, cmd] => UserCommand::Run(Some(node_id.to_string()), cmd.to_string()),
            _ => return Err(ParseCommandError::UnrecognizedCommand),
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseCommandError {
    #[error("data store disconnected")]
    UnrecognizedCommand,
}

unsafe impl Send for UserCommand {}

unsafe impl Sync for UserCommand {}

fn split_command_args(cmd: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chunk = String::new();
    let mut quote = false;
    for ch in cmd.chars() {
        match ch {
            ' ' if !quote => {
                if !chunk.is_empty() {
                    result.push(chunk);
                    chunk = String::new();
                }
            }
            '"' => {
                quote = !quote;
            }
            _ => {
                chunk.push(ch);
            }
        }
    }
    if !chunk.is_empty() {
        result.push(chunk);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::split_command_args;

    #[test]
    fn test_split_command() {
        assert_eq!(split_command_args("ls"), vec!["ls"]);
        assert_eq!(split_command_args(" ls "), vec!["ls"]);
        assert_eq!(split_command_args(" run ls "), vec!["run", "ls"]);
        assert_eq!(split_command_args("   run   ls   "), vec!["run", "ls"]);
        assert_eq!(
            split_command_args("   run   \" ls  \""),
            vec!["run", " ls  "]
        );
        assert_eq!(split_command_args(" run ls\"abc\" "), vec!["run", "lsabc"]);
        assert_eq!(split_command_args(" run ls\"\" "), vec!["run", "ls"]);
    }
}
