use bitcode::{Decode, Encode};
use std::{collections::BTreeMap, io::Cursor, net::Ipv4Addr};
use zstd::stream::decode_all;

use crate::util::time::now::std_now;
// open-source database bitcoded and then zstd'd
const GEO_IP_DB: &[u8; 14911018] = include_bytes!("./geoip.db");

#[derive(Encode, Decode, Debug, Clone)]
pub struct IpEntry {
    start: u32,
    end: u32,
    country: [u8; 2],
    lat: f64,
    lon: f64,
}

// TODO: Make the data and the info generic over v4 and v6 info
#[derive(serde_derive::Serialize, Clone)]
pub struct IpInfo {
    pub ip: Ipv4Addr,
    pub country_code: String,
    pub latitude: f64,
    pub longitude: f64,
}

pub fn decompress_and_deserialize() -> anyhow::Result<(BTreeMap<u32, IpEntry>, std::time::Duration)>
{
    let start = std_now();
    let cursor = Cursor::new(GEO_IP_DB);
    let decompressed_data = decode_all(cursor)?;
    let deserialized_map: BTreeMap<u32, IpEntry> = bitcode::decode(&decompressed_data)?;

    Ok((deserialized_map, start.elapsed()))
}

pub fn lookup_ip_location_from_map(
    geo_ip_db: &BTreeMap<u32, IpEntry>,
    ip: Ipv4Addr,
) -> Option<IpInfo> {
    let ip_as_u32 = u32::from(ip);
    for (_start, entry) in geo_ip_db.range(..=ip_as_u32).rev() {
        if entry.start <= ip_as_u32 && ip_as_u32 <= entry.end {
            let country_code = std::str::from_utf8(&entry.country).ok()?;
            return Some(IpInfo {
                ip,
                country_code: country_code.to_string(),
                latitude: entry.lat,
                longitude: entry.lon,
            });
        }
    }
    None
}
