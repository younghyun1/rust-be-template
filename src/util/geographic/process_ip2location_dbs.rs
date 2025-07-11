use bitcode::{Decode, Encode};
use std::io::Read;
use std::path::Path;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Write},
};

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IpRangeKey {
    V4(u32),
    V6(u128),
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct RawIpEntry {
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

#[derive(Encode, Decode, Debug, Clone)]
pub struct RawGeoIpBundle {
    entries: BTreeMap<IpRangeKey, RawIpEntry>,
}

fn main() {
    // Arguments: -ipv [4|6] <geocsv> <output-bundle>
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 || args[1] != "-ipv" || (args[2] != "4" && args[2] != "6") {
        eprintln!("Usage: {} -ipv [4|6] <geocsv> <output-bundle>", args[0]);
        std::process::exit(1);
    }
    let is_ipv6 = args[2] == "6";
    let input_path = &args[3];
    let output_path = &args[4];

    // 1) Load CSV into RAM
    let mut raw_csv_data = Vec::new();
    {
        let file = File::open(Path::new(input_path)).expect("cannot open input file");
        let mut reader = BufReader::new(file);
        reader
            .read_to_end(&mut raw_csv_data)
            .expect("read csv input");
    }
    let input_kib = raw_csv_data.len() as f64 / 1024.0;
    println!("Input file size: {input_kib:.2} KiB");

    // Parse CSV into RawIpEntry structs
    let mut raw_ip_map = BTreeMap::<IpRangeKey, RawIpEntry>::new();
    let reader = BufReader::new(&raw_csv_data[..]);
    for line in reader.lines() {
        let line = line.expect("read error");
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = vec![];
        let mut tmp = String::new();
        let mut quoted = false;
        for c in line.chars() {
            match c {
                '"' if !quoted => quoted = true,
                '"' if quoted => quoted = false,
                ',' if !quoted => {
                    fields.push(tmp.clone());
                    tmp.clear();
                }
                _ => tmp.push(c),
            }
        }
        if !tmp.is_empty() {
            fields.push(tmp.clone());
        }
        if fields.len() < 9 {
            continue;
        }

        let (start, end) = if is_ipv6 {
            (
                IpRangeKey::V6(fields[0].parse::<u128>().unwrap_or(0)),
                IpRangeKey::V6(fields[1].parse::<u128>().unwrap_or(0)),
            )
        } else {
            (
                IpRangeKey::V4(fields[0].parse::<u32>().unwrap_or(0)),
                IpRangeKey::V4(fields[1].parse::<u32>().unwrap_or(0)),
            )
        };

        let country_code_str = fields[2].to_owned();
        let country_name_str = fields[3].to_owned();
        let state_str = fields[4].to_owned();
        let city_str = fields[5].to_owned();
        let lat = fields[6].parse::<f64>().unwrap_or(0.0);
        let lon = fields[7].parse::<f64>().unwrap_or(0.0);
        let postal_str = fields[8].to_owned();

        let raw_entry = RawIpEntry {
            start: start.clone(),
            end,
            country_code: country_code_str,
            country_name: country_name_str,
            state: state_str,
            city: city_str,
            lat,
            lon,
            postal: postal_str,
        };
        raw_ip_map.insert(start, raw_entry);
    }

    // 2) bitcode encode
    let raw_bundle = RawGeoIpBundle {
        entries: raw_ip_map,
    };
    let raw_encoded = bitcode::encode(&raw_bundle);
    let raw_encoded_kib = raw_encoded.len() as f64 / 1024.0;
    println!("Bitcode encoded size: {raw_encoded_kib:.2} KiB");

    // 3) zstd 22 compress
    let mut out_file = File::create(output_path).expect("cannot open output file");
    let mut encoder =
        zstd::Encoder::new(&mut out_file, 22).expect("couldn't produce zstd encoder fsr");
    encoder.multithread(num_cpus::get() as u32).ok();
    encoder.long_distance_matching(true).ok();
    encoder
        .write_all(&raw_encoded)
        .expect("write zstd exception");
    let _encoder = encoder.finish().expect("zstd finish exception");
    let out_file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
    let compressed_kib = out_file_size as f64 / 1024.0;
    println!("Compressed size: {compressed_kib:.2} KiB");
}
