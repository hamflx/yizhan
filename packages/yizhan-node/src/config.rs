use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct YiZhanNodeConfig {
    #[serde(default = "default_client")]
    pub(crate) client: YiZhanClientConfig,
    #[serde(default = "default_server")]
    pub(crate) server: YiZhanServerConfig,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct YiZhanServerConfig {
    #[serde(default = "default_server_listen")]
    pub(crate) listen: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct YiZhanClientConfig {
    #[serde(default = "default_client_host")]
    pub(crate) host: String,
    #[serde(default = "default_client_port")]
    pub(crate) port: u16,
}

fn default_client() -> YiZhanClientConfig {
    YiZhanClientConfig {
        host: default_client_host(),
        port: default_client_port(),
    }
}

fn default_server() -> YiZhanServerConfig {
    YiZhanServerConfig {
        listen: default_server_listen(),
    }
}

fn default_server_listen() -> String {
    format!("127.0.0.1:{}", default_client_port())
}

fn default_client_host() -> String {
    "127.0.0.1".to_string()
}

fn default_client_port() -> u16 {
    3777
}
