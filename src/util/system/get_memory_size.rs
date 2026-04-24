use tracing::error;

pub fn get_memory_size() -> u64 {
    platform_memory_size()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn platform_memory_size() -> u64 {
    let page_count = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };

    if page_count <= 0 || page_size <= 0 {
        error!(
            page_count,
            page_size, "Failed to query physical memory size with sysconf"
        );
        return 0;
    }

    let page_count = match u64::try_from(page_count) {
        Ok(page_count) => page_count,
        Err(e) => {
            error!(page_count, error = ?e, "Failed to convert physical page count");
            return 0;
        }
    };
    let page_size = match u64::try_from(page_size) {
        Ok(page_size) => page_size,
        Err(e) => {
            error!(page_size, error = ?e, "Failed to convert physical page size");
            return 0;
        }
    };

    page_count.saturating_mul(page_size)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn platform_memory_size() -> u64 {
    error!("Physical memory size lookup is unsupported on this target");
    0
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
        assert!(
            mem > 128 * 1024 * 1024,
            "Memory size too small: {} bytes",
            mem
        );
    }
}
