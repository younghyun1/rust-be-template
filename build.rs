use std::fs;
use std::fs::File;
use std::io::Write;

fn main() {
    let build_time = chrono::Utc::now().to_rfc3339();

    let cargo_lock = fs::read_to_string("Cargo.lock").expect("Cargo.lock not found");
    let version = cargo_lock
        .lines()
        .collect::<Vec<_>>()
        .windows(4)
        .find(|window| {
            window[0].trim() == "[[package]]"
                && window[1].trim() == "name = \"axum\""
                && window[2].trim().starts_with("version = ")
        })
        .and_then(|window| {
            window[2]
                .trim()
                .strip_prefix("version = ")
                .and_then(|v| v.trim_matches('"').into())
        })
        .map(|v: &str| v.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let rust_version = std::process::Command::new("cargo")
        .args(["+nightly", "--version"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let mut file = File::create("src/build_info.rs").unwrap();
    writeln!(file, "pub const BUILD_TIME: &str = \"{}\";", build_time).unwrap();
    writeln!(
        file,
        "pub const AXUM_VERSION: &str = \"axum {}\";",
        version
    )
    .unwrap();
    writeln!(
        file,
        "pub const RUST_VERSION: &str = \"{}\";",
        rust_version
    )
    .unwrap();
}
