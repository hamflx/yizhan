use yizhan_protocol::command::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestCommand(pub(crate) Option<String>, pub(crate) Command);
