use yizhan_protocol::command::{UserCommand, UserCommandResponse, UserCommandResult};

pub trait Plugin: Send + Sync {
    fn parse_command(&self, inputs: &[&str]) -> Option<(Option<String>, UserCommand)>;

    fn execute_command(&self, group_id: &str, content: &str) -> Option<UserCommandResult>;

    fn show_response(&self, _response: &UserCommandResponse) -> Option<String> {
        None
    }
}
