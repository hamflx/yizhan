use std::sync::Arc;

use tokio::sync::Mutex;
use yizhan_plugin::Plugin;

pub(crate) struct PluginManagement {
    pub(crate) plugins: Arc<Mutex<Vec<Box<dyn Plugin>>>>,
}

impl PluginManagement {
    pub(crate) fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) async fn add_plugin(&self, plugin: Box<dyn Plugin>) {
        let mut lock = self.plugins.lock().await;
        lock.push(plugin);
    }
}
