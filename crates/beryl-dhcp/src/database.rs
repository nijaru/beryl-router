use crate::server::{PoolConfig, StaticLease};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lease {
    pub ip: Ipv4Addr,
    pub mac: Vec<u8>,
    pub expires_at: SystemTime,
    pub hostname: Option<String>,
}

pub struct LeaseDatabase {
    leases: HashMap<Vec<u8>, Lease>, // MAC -> Lease
    pool_start: u32,
    pool_end: u32,
    static_leases: HashMap<Vec<u8>, Ipv4Addr>,
    lease_duration: Duration,
    storage_path: Option<PathBuf>,
}

impl LeaseDatabase {
    pub fn new(
        pool: &PoolConfig,
        static_leases: &[StaticLease],
        storage_path: Option<PathBuf>,
    ) -> Self {
        let mut sl_map = HashMap::new();
        for sl in static_leases {
            if let Ok(mac_bytes) = parse_mac(&sl.mac) {
                sl_map.insert(mac_bytes, sl.ip);
            }
        }

        let lease_duration =
            parse_duration(&pool.lease_time).unwrap_or(Duration::from_secs(12 * 3600));

        let mut db = Self {
            leases: HashMap::new(),
            pool_start: u32::from(pool.start),
            pool_end: u32::from(pool.end),
            static_leases: sl_map,
            lease_duration,
            storage_path,
        };

        if let Err(e) = db.load() {
            tracing::warn!("Failed to load leases: {}", e);
        }

        db
    }

    pub fn load(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.storage_path {
            if path.exists() {
                let content = fs::read_to_string(path)?;
                let stored_leases: Vec<Lease> = serde_json::from_str(&content)?;
                for lease in stored_leases {
                    // Only keep valid leases? Or keep expired ones too?
                    // For now, load everything.
                    self.leases.insert(lease.mac.clone(), lease);
                }
                tracing::info!("Loaded {} leases from storage", self.leases.len());
            }
        }
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = &self.storage_path {
            let leases_vec: Vec<&Lease> = self.leases.values().collect();
            let content = serde_json::to_string_pretty(&leases_vec)?;
            fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn get_lease(&self, mac: &[u8]) -> Option<&Lease> {
        self.leases.get(mac)
    }

    pub fn allocate_ip(&mut self, mac: &[u8], requested_ip: Option<Ipv4Addr>) -> Option<Lease> {
        let lease = self.allocate_ip_internal(mac, requested_ip)?;

        if let Err(e) = self.save() {
            tracing::error!("Failed to persist lease: {}", e);
        }

        Some(lease)
    }

    fn allocate_ip_internal(
        &mut self,
        mac: &[u8],
        requested_ip: Option<Ipv4Addr>,
    ) -> Option<Lease> {
        // 1. Check static leases
        if let Some(&ip) = self.static_leases.get(mac) {
            let lease = Lease {
                ip,
                mac: mac.to_vec(),
                expires_at: SystemTime::now() + self.lease_duration,
                hostname: None,
            };
            self.leases.insert(mac.to_vec(), lease.clone());
            return Some(lease);
        }

        // 2. Check existing lease
        if let Some(lease) = self.leases.get(mac) {
            let mut new_lease = lease.clone();
            new_lease.expires_at = SystemTime::now() + self.lease_duration;
            self.leases.insert(mac.to_vec(), new_lease.clone());
            return Some(new_lease);
        }

        // 3. Allocate new dynamic IP
        // Try requested IP first if in pool and available
        if let Some(req) = requested_ip {
            let req_u32 = u32::from(req);
            if req_u32 >= self.pool_start && req_u32 <= self.pool_end && !self.is_ip_taken(req) {
                let lease = Lease {
                    ip: req,
                    mac: mac.to_vec(),
                    expires_at: SystemTime::now() + self.lease_duration,
                    hostname: None,
                };
                self.leases.insert(mac.to_vec(), lease.clone());
                return Some(lease);
            }
        }

        // Find first free IP
        for ip_u32 in self.pool_start..=self.pool_end {
            let ip = Ipv4Addr::from(ip_u32);
            if !self.is_ip_taken(ip) {
                let lease = Lease {
                    ip,
                    mac: mac.to_vec(),
                    expires_at: SystemTime::now() + self.lease_duration,
                    hostname: None,
                };
                self.leases.insert(mac.to_vec(), lease.clone());
                return Some(lease);
            }
        }

        None // Pool exhausted
    }

    fn is_ip_taken(&self, ip: Ipv4Addr) -> bool {
        // Check active leases
        if self
            .leases
            .values()
            .any(|l| l.ip == ip && l.expires_at > SystemTime::now())
        {
            return true;
        }
        // Check static leases (reverse check)
        if self.static_leases.values().any(|&sip| sip == ip) {
            return true;
        }
        false
    }

    pub fn get_duration(&self) -> Duration {
        self.lease_duration
    }

    pub fn get_ip_by_hostname(&self, hostname: &str) -> Option<Ipv4Addr> {
        // Check static leases
        for lease in self.static_leases.iter() {
            // Static leases map is MAC -> IP. We don't have hostname in the map value.
            // We need to look up the original config or store hostname in static lease map.
            // Current static_leases map is HashMap<Vec<u8>, Ipv4Addr>.
            // We need to change the map value or iterate the source list if possible.
            // But we don't have the source list here.
            // Let's check dynamic/active leases first as they are full Lease objects.
            if let Some(l) = self.leases.get(lease.0) {
                if let Some(h) = &l.hostname {
                    if h.eq_ignore_ascii_case(hostname) {
                        return Some(l.ip);
                    }
                }
            }
        }

        // Check all active leases
        for lease in self.leases.values() {
            if let Some(h) = &lease.hostname {
                if h.eq_ignore_ascii_case(hostname) {
                    return Some(lease.ip);
                }
            }
        }

        None
    }
}

fn parse_mac(s: &str) -> Result<Vec<u8>, ()> {
    let bytes: Result<Vec<u8>, _> = s
        .split(':')
        .map(|part| u8::from_str_radix(part, 16))
        .collect();
    bytes.map_err(|_| ())
}

fn parse_duration(s: &str) -> Option<Duration> {
    // Basic parsing: "12h", "30m", "60s"
    let len = s.len();
    if len < 2 {
        return None;
    }
    let (val, unit) = s.split_at(len - 1);
    let val = val.parse::<u64>().ok()?;
    match unit {
        "h" => Some(Duration::from_secs(val * 3600)),
        "m" => Some(Duration::from_secs(val * 60)),
        "s" => Some(Duration::from_secs(val)),
        _ => None,
    }
}
