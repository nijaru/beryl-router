use crate::server::{PoolConfig, StaticLease};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::{SystemTime, Duration};

#[derive(Clone, Debug)]
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
}

impl LeaseDatabase {
    pub fn new(pool: &PoolConfig, static_leases: &[StaticLease]) -> Self {
        let mut sl_map = HashMap::new();
        for sl in static_leases {
            // Simple hex string to bytes parsing (assuming AA:BB:CC...)
            if let Ok(mac_bytes) = parse_mac(&sl.mac) {
                sl_map.insert(mac_bytes, sl.ip);
            }
        }

        // Parse lease time (very basic for now)
        let lease_duration = parse_duration(&pool.lease_time).unwrap_or(Duration::from_secs(12 * 3600));

        Self {
            leases: HashMap::new(),
            pool_start: u32::from(pool.start),
            pool_end: u32::from(pool.end),
            static_leases: sl_map,
            lease_duration,
        }
    }

    pub fn get_lease(&self, mac: &[u8]) -> Option<&Lease> {
        self.leases.get(mac)
    }

    pub fn allocate_ip(&mut self, mac: &[u8], requested_ip: Option<Ipv4Addr>) -> Option<Lease> {
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
        if self.leases.values().any(|l| l.ip == ip && l.expires_at > SystemTime::now()) {
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
}

fn parse_mac(s: &str) -> Result<Vec<u8>, ()> {
    let bytes: Result<Vec<u8>, _> = s.split(':')
        .map(|part| u8::from_str_radix(part, 16))
        .collect();
    bytes.map_err(|_| ())
}

fn parse_duration(s: &str) -> Option<Duration> {
    // Basic parsing: "12h", "30m", "60s"
    let len = s.len();
    if len < 2 { return None; }
    let (val, unit) = s.split_at(len - 1);
    let val = val.parse::<u64>().ok()?;
    match unit {
        "h" => Some(Duration::from_secs(val * 3600)),
        "m" => Some(Duration::from_secs(val * 60)),
        "s" => Some(Duration::from_secs(val)),
        _ => None,
    }
}
