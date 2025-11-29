use crate::server::{PoolConfig, StaticLease};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lease {
    pub mac: String,
    pub ip: Ipv4Addr,
    pub hostname: Option<String>,
    pub expires_at: SystemTime,
}

pub struct LeaseDatabase {
    leases: HashMap<Ipv4Addr, Lease>,
    pool: PoolConfig,
    storage_path: Option<PathBuf>,
    static_leases: HashMap<String, Ipv4Addr>, // MAC -> IP
}

impl LeaseDatabase {
    #[must_use]
    pub fn new(pool: PoolConfig, storage_path: Option<PathBuf>, static_leases_vec: &[StaticLease]) -> Self {
        let mut static_leases = HashMap::new();
        for sl in static_leases_vec {
            static_leases.insert(sl.mac.to_lowercase(), sl.ip);
        }

        Self {
            leases: HashMap::new(),
            pool,
            storage_path,
            static_leases,
        }
    }

    #[must_use]
    pub fn available(&self, ip: Ipv4Addr) -> bool {
        // Check if in pool range
        if ip < self.pool.start || ip > self.pool.end {
            return false;
        }

        // Check if taken by static lease
        if self.static_leases.values().any(|&x| x == ip) {
            return false;
        }

        // Check if active dynamic lease exists
        if let Some(lease) = self.leases.get(&ip) 
            && lease.expires_at > SystemTime::now() {
            return false;
        }

        true
    }

    /// Load leases from persistent storage
    ///
    /// # Errors
    ///
    /// Returns an error if the storage file exists but cannot be read or if the
    /// JSON content is invalid.
    pub fn load(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.storage_path 
            && path.exists() 
        {
            let content = fs::read_to_string(path)?;
            let stored_leases: Vec<Lease> = serde_json::from_str(&content)?;
            
            for lease in stored_leases {
                // Only load valid leases
                if lease.expires_at > SystemTime::now() {
                    self.leases.insert(lease.ip, lease);
                }
            }
            tracing::info!("Loaded {} leases from storage", self.leases.len());
        }
        Ok(())
    }

    /// Save current leases to persistent storage
    ///
    /// # Errors
    ///
    /// Returns an error if the storage path is defined but the file cannot be written
    /// or if the lease data cannot be serialized to JSON.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = &self.storage_path {
            // Prune expired before saving
            let valid_leases: Vec<&Lease> = self.leases.values()
                .filter(|l| l.expires_at > SystemTime::now())
                .collect();
            
            let content = serde_json::to_string_pretty(&valid_leases)?;
            fs::write(path, content)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn get_lease(&self, mac: &[u8]) -> Option<&Lease> {
        let mac_str = mac_to_string(mac);
        self.leases.values().find(|l| l.mac == mac_str)
    }

    /// Allocates an IP for the given MAC address.
    ///
    /// If the MAC has a static lease, that IP is returned.
    /// If `requested_ip` is provided and valid/available, it is used.
    /// Otherwise, the next available IP in the pool is assigned.
    ///
    /// Returns `None` if the pool is exhausted.
    pub fn allocate_ip(&mut self, mac: &[u8], requested_ip: Option<Ipv4Addr>) -> Option<Lease> {
        let mac_str = mac_to_string(mac);
        let duration = Self::parse_duration(&self.pool.lease_time);
        
        // 1. Check static leases
        let ip = if let Some(&static_ip) = self.static_leases.get(&mac_str) {
            static_ip
        } else if let Some(req) = requested_ip 
             && self.available(req) {
            // 2. Try requested IP
            req
        } else {
            // 3. Pick next available
            let mut current: u32 = self.pool.start.into();
            let end: u32 = self.pool.end.into();
            let mut found = None;

            while current <= end {
                let candidate = Ipv4Addr::from(current);
                if self.available(candidate) {
                    found = Some(candidate);
                    break;
                }
                current += 1;
            }
            found?
        };

        let lease = Lease {
            mac: mac_str,
            ip,
            hostname: None, // Hostname is updated separately if needed
            expires_at: SystemTime::now() + duration,
        };

        self.leases.insert(ip, lease.clone());
        if let Err(e) = self.save() {
            tracing::error!("Failed to save lease database: {}", e);
        }

        Some(lease)
    }

    fn parse_duration(s: &str) -> Duration {
        // Simple parser: "12h", "30m", "3600"
        if let Some(stripped) = s.strip_suffix('h') 
            && let Ok(hours) = stripped.parse::<u64>() {
            return Duration::from_secs(hours * 3600);
        }
        if let Some(stripped) = s.strip_suffix('m') 
            && let Ok(mins) = stripped.parse::<u64>() {
            return Duration::from_secs(mins * 60);
        }
        if let Ok(secs) = s.parse::<u64>() {
            return Duration::from_secs(secs);
        }
        Duration::from_secs(3600) // Default 1h
    }

    #[must_use]
    pub fn get_duration(&self) -> Duration {
        Self::parse_duration(&self.pool.lease_time)
    }

    #[must_use]
    pub fn get_ip_by_hostname(&self, hostname: &str) -> Option<Ipv4Addr> {
        // Check static leases first
        for lease in &self.static_leases {
             if let Some(l) = self.leases.get(lease.1) 
                 && let Some(h) = &l.hostname 
                 && h.eq_ignore_ascii_case(hostname) 
             {
                    return Some(l.ip);
             }
        }

        // Check dynamic leases
        for lease in self.leases.values() {
            if let Some(h) = &lease.hostname 
                && h.eq_ignore_ascii_case(hostname) 
            {
                return Some(lease.ip);
            }
        }

        None
    }
}

fn mac_to_string(mac: &[u8]) -> String {
    mac.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}