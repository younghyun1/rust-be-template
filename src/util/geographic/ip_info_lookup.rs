use bitcode::Decode;
use internment::Intern;
use std::{collections::BTreeMap, fs::File, io::BufReader, net::IpAddr, path::Path};
use zstd::stream::decode_all;

use crate::util::time::now::std_now;

/// same as before
#[derive(Decode, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IpRangeKey {
    V4(u32),
    V6(u128),
}

/// this is just the raw, un‐interned thing that `bitcode::Decode` fills for us
#[derive(Decode, Debug, Clone)]
struct RawIpEntry {
    start: IpRangeKey,
    end: IpRangeKey,
    country_code: String,
    country_name: String,
    state: String,
    city: String,
    lat: f64,
    lon: f64,
    postal: String,
}

#[derive(Decode, Debug, Clone)]
struct RawGeoIpBundle {
    entries: BTreeMap<IpRangeKey, RawIpEntry>,
}

/// this is exposed so that the `State` struct can hold it in memory:
/// Note: start/end are stored as the BTreeMap key, no need to duplicate here
#[derive(Debug, Clone)]
pub struct IpEntry {
    pub end: IpRangeKey,
    pub country_code: Intern<String>,
    pub country_name: Intern<String>,
    pub state: Intern<String>,
    pub city: Intern<String>,
    pub postal: Intern<String>,
    pub latitude: f64,
    pub longitude: f64,
}

impl IpEntry {
    fn contains_ip(&self, ip: IpAddr) -> bool {
        match (ip, &self.end) {
            (IpAddr::V4(addr), IpRangeKey::V4(end)) => u32::from(addr) <= *end,
            (IpAddr::V6(addr), IpRangeKey::V6(end)) => u128::from(addr) <= *end,
            // Mismatched IP versions can't be in the range.
            _ => false,
        }
    }
}

/// unchanged public lookup result
#[derive(serde::Serialize, Clone)]
pub struct IpInfo {
    pub ip: IpAddr,
    pub country_code: String,
    pub country_name: String,
    pub state: String,
    pub city: String,
    pub postal: String,
    pub latitude: f64,
    pub longitude: f64,
}

/// hold both v4 and v6 maps
pub struct GeoIpDatabases {
    pub v4: BTreeMap<IpRangeKey, IpEntry>,
    pub v6: BTreeMap<IpRangeKey, IpEntry>,
}

/// 1) decompress & bitcode‐decode into RawGeoIpBundle
/// 2) immediately convert every RawIpEntry → IpEntry, interning all the strings
pub fn decompress_and_deserialize() -> anyhow::Result<(GeoIpDatabases, std::time::Duration)> {
    let start = std_now();

    // Process v4 file in its own scope to ensure cleanup
    let v4_interned = {
        let file = match File::open(Path::new("./new_bundle_ipv4.db")) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to open ./new_bundle_ipv4.db");
                return Err(e.into());
            }
        };
        let decompressed = decode_all(BufReader::new(file))?;
        let raw: RawGeoIpBundle = bitcode::decode(&decompressed)?;
        drop(decompressed);

        raw.entries
            .into_iter()
            .map(|(k, raw)| {
                let ie = IpEntry {
                    end: raw.end,
                    country_code: Intern::new(raw.country_code),
                    country_name: Intern::new(raw.country_name),
                    state: Intern::new(raw.state),
                    city: Intern::new(raw.city),
                    postal: Intern::new(raw.postal),
                    latitude: raw.lat,
                    longitude: raw.lon,
                };
                (k, ie)
            })
            .collect()
    };

    // Process v6 file in its own scope (v4 raw data is already dropped)
    let v6_interned = {
        let file = match File::open(Path::new("./new_bundle_ipv6.db")) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to open ./new_bundle_ipv6.db");
                return Err(e.into());
            }
        };
        let decompressed = decode_all(BufReader::new(file))?;
        let raw: RawGeoIpBundle = bitcode::decode(&decompressed)?;
        drop(decompressed);

        raw.entries
            .into_iter()
            .map(|(k, raw)| {
                let ie = IpEntry {
                    end: raw.end,
                    country_code: Intern::new(raw.country_code),
                    country_name: Intern::new(raw.country_name),
                    state: Intern::new(raw.state),
                    city: Intern::new(raw.city),
                    postal: Intern::new(raw.postal),
                    latitude: raw.lat,
                    longitude: raw.lon,
                };
                (k, ie)
            })
            .collect()
    };

    let dbs = GeoIpDatabases {
        v4: v4_interned,
        v6: v6_interned,
    };
    Ok((dbs, start.elapsed()))
}

pub fn lookup_ip_location_from_map(geo: &GeoIpDatabases, ip: IpAddr) -> Option<IpInfo> {
    let candidate = match ip {
        IpAddr::V4(addr) => {
            let key = IpRangeKey::V4(addr.into());
            geo.v4.range(..=key).next_back()
        }
        IpAddr::V6(addr) => {
            let key = IpRangeKey::V6(addr.into());
            geo.v6.range(..=key).next_back()
        }
    };

    candidate
        .filter(|(_start_key, entry)| entry.contains_ip(ip))
        .map(|(_start_key, entry)| IpInfo {
            ip,
            country_code: entry.country_code.to_string(),
            country_name: entry.country_name.to_string(),
            state: entry.state.to_string(),
            city: entry.city.to_string(),
            postal: entry.postal.to_string(),
            latitude: entry.latitude,
            longitude: entry.longitude,
        })
}
