use yizhan_protocol::version::VersionInfo;

use crate::config::YiZhanNodeConfig;

pub(crate) struct YiZhanContext {
    pub(crate) name: String,
    pub(crate) version: VersionInfo,
    pub(crate) server_mode: bool,
    pub(crate) config: YiZhanNodeConfig,
}
