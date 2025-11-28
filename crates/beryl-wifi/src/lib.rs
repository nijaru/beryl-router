pub mod manager;
pub mod uci;

use crate::manager::WifiManager;
use beryl_config::WifiConfig;

pub async fn apply_wifi_config(config: &WifiConfig) -> anyhow::Result<()> {
    let manager = WifiManager::new();
    manager.apply_config(config).await
}
