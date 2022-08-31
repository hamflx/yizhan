use std::str::FromStr;

use yizhan_protocol::command::UserCommand;

pub(crate) mod run;

#[derive(thiserror::Error, Debug)]
pub enum ParseCommandError {
    #[error("data store disconnected")]
    UnrecognizedCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestCommand(pub(crate) Option<String>, pub(crate) UserCommand);

impl FromStr for RequestCommand {
    type Err = ParseCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = split_command_args(s);
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        let args = args.as_slice();

        Ok(match args {
            &["update"] => RequestCommand(None, UserCommand::Update),
            &["run", cmd] => RequestCommand(None, UserCommand::Run(cmd.to_string())),
            &["run", node_id, cmd] => {
                RequestCommand(Some(node_id.to_string()), UserCommand::Run(cmd.to_string()))
            }
            _ => return Err(ParseCommandError::UnrecognizedCommand.into()),
        })
    }
}

pub(crate) fn split_command_args(cmd: &str) -> Vec<String> {
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
