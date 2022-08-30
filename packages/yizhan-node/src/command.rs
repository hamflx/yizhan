use yizhan_protocol::command::UserCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestCommand(pub(crate) Option<String>, pub(crate) UserCommand);
