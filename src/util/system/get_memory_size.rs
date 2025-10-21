pub fn get_memory_size() -> u64 {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::SystemInformation::GetPhysicallyInstalledSystemMemory;
        let mut kilobytes: u64 = 0;
        unsafe {
            GetPhysicallyInstalledSystemMemory(&mut kilobytes).unwrap();
        }

        kilobytes * 1024 // bytes
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let meminfo = File::open("/proc/meminfo").unwrap();
        let reader = BufReader::new(meminfo);

        for line in reader.lines() {
            let line = line.unwrap();
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let kb: u64 = parts[1].parse().unwrap();
                return kb * 1024; // bytes
            }
        }

        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_get_memory_size_nonzero() {
        let start = Instant::now();
        let mem = get_memory_size();
        let duration = start.elapsed();
        println!("get_memory_size() took {:?}", duration);
        // Should be at least 128 MB on modern systems
        assert!(
            mem > 128 * 1024 * 1024,
            "Memory size too small: {} bytes",
            mem
        );
    }
}
