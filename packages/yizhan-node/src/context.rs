use yizhan_protocol::version::VersionInfo;

pub(crate) struct YiZhanContext {
    pub(crate) name: String,
    pub(crate) version: VersionInfo,
    pub(crate) server_mode: bool,
}
