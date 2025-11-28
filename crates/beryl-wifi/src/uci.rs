use beryl_config::{WifiConfig, WifiInterfaceConfig, WifiRadioConfig};
use std::fmt::Write;

pub struct UciGenerator;

impl UciGenerator {
    pub fn generate(config: &WifiConfig) -> String {
        let mut out = String::new();

        // Radio 0 (2.4GHz usually, depends on HW)
        if let Some(radio) = &config.radio0 {
            Self::write_radio(&mut out, "radio0", radio);
        }

        // Radio 1 (5GHz usually)
        if let Some(radio) = &config.radio1 {
            Self::write_radio(&mut out, "radio1", radio);
        }

        // Interfaces
        for (idx, iface) in config.interfaces.iter().enumerate() {
            Self::write_iface(&mut out, idx, iface);
        }

        out
    }

    fn write_radio(out: &mut String, name: &str, radio: &WifiRadioConfig) {
        writeln!(out, "config wifi-device '{}'", name).unwrap();
        writeln!(out, "\toption type 'mac80211'").unwrap();
        writeln!(out, "\toption path '{}'", radio.path).unwrap();
        writeln!(out, "\toption channel '{}'", radio.channel).unwrap();
        writeln!(out, "\toption band '{}'", radio.band).unwrap();
        writeln!(out, "\toption htmode '{}'", radio.htmode).unwrap();
        if !radio.disabled {
            writeln!(out, "\toption disabled '0'").unwrap();
        } else {
            writeln!(out, "\toption disabled '1'").unwrap();
        }
        writeln!(out).unwrap();
    }

    fn write_iface(out: &mut String, idx: usize, iface: &WifiInterfaceConfig) {
        writeln!(out, "config wifi-iface 'default_{}_{}'", iface.device, idx).unwrap();
        writeln!(out, "\toption device '{}'", iface.device).unwrap();
        writeln!(out, "\toption network '{}'", iface.network).unwrap();
        writeln!(out, "\toption mode '{}'", iface.mode).unwrap();
        writeln!(out, "\toption ssid '{}'", iface.ssid).unwrap();
        writeln!(out, "\toption encryption '{}'", iface.encryption).unwrap();
        writeln!(out, "\toption key '{}'", iface.key).unwrap();
        writeln!(out).unwrap();
    }
}
