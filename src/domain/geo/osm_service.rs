use std::net::{IpAddr, Ipv4Addr};

use anyhow::Result;
use serde_derive::Deserialize;

use crate::{init::state::ServerState, util::geographic::ip_info_lookup::IpInfo};

#[derive(Deserialize)]
pub struct OsmResponse {
    pub place_id: Option<i64>,
    pub licence: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i64>,
    pub lat: Option<String>,
    pub lon: Option<String>,
    pub class: Option<String>,
    #[serde(rename = "type")]
    pub osm_type_detail: Option<String>,
    pub place_rank: Option<i32>,
    pub importance: Option<f64>,
    pub addresstype: Option<String>,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub address: Option<OsmAddress>,
    pub boundingbox: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct OsmAddress {
    pub amenity: Option<String>,
    pub house_number: Option<String>,
    pub road: Option<String>,
    pub neighbourhood: Option<String>,
    pub suburb: Option<String>,
    pub city: Option<String>,
    pub county: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "ISO3166-2-lvl4")]
    pub iso3166_2_lvl4: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<String>,      // english
    pub country_code: Option<String>, // two-letter
}

pub async fn get_osm_data_for_ip_addr(
    lat: f64,
    lon: f64,
    client: &reqwest::Client,
) -> Result<(String, String)> {
    let response = client
        .get(format!(
            "https://nominatim.openstreetmap.org/reverse?format=json&lat={lat}&lon={lon}"
        ))
        .send()
        .await?;

    let response = response.error_for_status()?;
    let text = response.text().await?;
    let osm_response: OsmResponse = serde_json::from_str(&text)?;

    let city = osm_response
        .address
        .as_ref()
        .and_then(|addr| addr.city.as_ref())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing city in OSM response"))?;

    let country = osm_response
        .address
        .as_ref()
        .and_then(|addr| addr.country.as_ref())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing country in OSM response"))?;

    Ok((city, country))
}

#[derive(Deserialize)]
pub struct OsmSearchResponse {
    pub place_id: Option<i64>,
    pub licence: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i64>,
    pub lat: Option<String>,
    pub lon: Option<String>,
    pub class: Option<String>,
    #[serde(rename = "type")]
    pub osm_type_detail: Option<String>,
    pub place_rank: Option<i32>,
    pub importance: Option<f64>,
    pub addresstype: Option<String>,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub boundingbox: Option<Vec<String>>,
}

pub async fn city_country_to_lat_lon(
    city: &str,
    country: &str,
    client: &reqwest::Client,
) -> Result<(f64, f64)> {
    let response = client
        .get("https://nominatim.openstreetmap.org/search")
        .query(&[("city", city), ("country", country), ("format", "json")])
        .send()
        .await?;

    let response = response.error_for_status()?;
    let text = response.text().await?;
    let results: Vec<OsmSearchResponse> = serde_json::from_str(&text)?;
    let first = results
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No results returned from OSM"))?;

    let lat = first
        .lat
        .ok_or_else(|| anyhow::anyhow!("Missing lat in OSM search result"))?
        .parse::<f64>()
        .map_err(|e| anyhow::anyhow!("Failed to parse lat: {}", e))?;
    let lon = first
        .lon
        .ok_or_else(|| anyhow::anyhow!("Missing lon in OSM search result"))?
        .parse::<f64>()
        .map_err(|e| anyhow::anyhow!("Failed to parse lon: {}", e))?;
    Ok((lat, lon))
}

pub async fn get_lat_lon_for_ip_addr(ip_addr: &str, state: &ServerState) -> Result<(f64, f64)> {
    let ipv4_addr = ip_addr
        .parse::<Ipv4Addr>()
        .map_err(|e| anyhow::anyhow!("Failed to parse IP address: {}", e))?;
    let ip_addr_lat_lon: IpInfo = state
        .lookup_ip_location(IpAddr::V4(ipv4_addr))
        .ok_or_else(|| anyhow::anyhow!("Could not find location for IP address"))?;
    let client = state.get_request_client();

    let (city, country) =
        get_osm_data_for_ip_addr(ip_addr_lat_lon.latitude, ip_addr_lat_lon.longitude, client)
            .await?;

    let (fin_lat, fin_lon) = city_country_to_lat_lon(&city, &country, client).await?;

    Ok((fin_lat, fin_lon))
}
