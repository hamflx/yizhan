use yizhan_protocol::command::{UserCommandResponse, UserCommandResult};

use crate::plugins::PluginManagement;

pub(crate) mod local;
pub(crate) mod remote;

pub(crate) async fn show_response(
    response: UserCommandResult,
    plugins: &PluginManagement,
) -> String {
    let plugins = plugins.plugins.lock().await;
    match response {
        UserCommandResult::Ok(UserCommandResponse::Get(content)) => {
            String::from_utf8(content).ok().unwrap_or_default()
        }
        UserCommandResult::Ok(response) => plugins
            .iter()
            .find_map(|p| p.show_response(&response))
            .unwrap_or_else(|| format!("Response: {:#?}\n", response)),
        _ => format!("Response: {:#?}\n", response),
    }
}
