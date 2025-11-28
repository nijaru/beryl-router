use crate::uci::UciGenerator;
use anyhow::{Context, Result};
use beryl_config::WifiConfig;
use std::path::Path;
use tokio::process::Command;

pub struct WifiManager;

impl WifiManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn apply_config(&self, config: &WifiConfig) -> Result<()> {
        tracing::info!("Applying WiFi configuration...");

        // 1. Generate UCI config content
        let uci_content = UciGenerator::generate(config);

        // 2. Write to /etc/config/wireless
        // For dev/testing, we might write to a temp file if not on router
        let target_path = if Path::new("/etc/config").exists() {
            "/etc/config/wireless"
        } else {
            "target/wireless_config_preview"
        };

        std::fs::write(target_path, uci_content)
            .with_context(|| format!("Failed to write WiFi config to {}", target_path))?;

        tracing::info!("Wrote WiFi config to {}", target_path);

        // 3. Reload WiFi if on router
        if Path::new("/sbin/wifi").exists() {
            tracing::info!("Reloading WiFi subsystem...");
            let output = Command::new("/sbin/wifi").arg("reload").output().await?;

            if !output.status.success() {
                tracing::error!(
                    "'wifi reload' failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return Err(anyhow::anyhow!("Failed to reload WiFi"));
            }
            tracing::info!("WiFi reloaded successfully");
        } else {
            tracing::debug!("Skipping 'wifi reload' (not on router)");
        }

        Ok(())
    }
}
