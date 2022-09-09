use tracing::info;
use yizhan_plugin::Plugin;
use yizhan_protocol::command::UserCommand;

#[derive(Default)]
pub struct YiZhanPowerOffPlugin {}

impl Plugin for YiZhanPowerOffPlugin {
    fn parse_command(&self, inputs: &[&str]) -> Option<(Option<String>, UserCommand)> {
        match inputs {
            ["poweroff"] => Some((
                None,
                UserCommand::PluginCommand("poweroff".to_string(), "poweroff".to_string()),
            )),
            ["poweroff", host] => Some((
                Some(host.to_string()),
                UserCommand::PluginCommand("poweroff".to_string(), "poweroff".to_string()),
            )),
            _ => None,
        }
    }

    fn execute_command(&self, group_id: &str, content: &str) {
        if matches!((group_id, content), ("poweroff", "poweroff")) {
            info!("Shutting down ...");
        }
    }
}
