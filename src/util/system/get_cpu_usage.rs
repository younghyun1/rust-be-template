pub async fn get_cpu_usage() -> f64 {
    #[cfg(target_os = "windows")]
    {
        use std::{mem::zeroed, time::Duration};
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::GetSystemTimes;

        unsafe fn filetime_to_u64(ft: &FILETIME) -> u64 {
            ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
        }

        unsafe {
            let mut idle_time1: FILETIME = zeroed();
            let mut kernel_time1: FILETIME = zeroed();
            let mut user_time1: FILETIME = zeroed();

            // Use GetSystemTimes, not GetSystemTime
            if GetSystemTimes(
                Some(&mut idle_time1),
                Some(&mut kernel_time1),
                Some(&mut user_time1),
            )
            .is_err()
            {
                return 0.0;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;

            let mut idle_time2: FILETIME = zeroed();
            let mut kernel_time2: FILETIME = zeroed();
            let mut user_time2: FILETIME = zeroed();

            if GetSystemTimes(
                Some(&mut idle_time2),
                Some(&mut kernel_time2),
                Some(&mut user_time2),
            )
            .is_err()
            {
                return 0.0;
            }

            let idle1 = filetime_to_u64(&idle_time1);
            let idle2 = filetime_to_u64(&idle_time2);
            let kernel1 = filetime_to_u64(&kernel_time1);
            let kernel2 = filetime_to_u64(&kernel_time2);
            let user1 = filetime_to_u64(&user_time1);
            let user2 = filetime_to_u64(&user_time2);

            let sys1 = kernel1 + user1;
            let sys2 = kernel2 + user2;

            let sys_delta = sys2.saturating_sub(sys1);
            let idle_delta = idle2.saturating_sub(idle1);

            if sys_delta == 0 {
                0.0
            } else {
                ((sys_delta - idle_delta) as f64) * 100.0 / (sys_delta as f64)
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::{
            fs::File,
            io::{BufRead, BufReader},
            time::Duration,
        };

        fn read_proc_stat() -> Option<(u64, u64)> {
            let file = File::open("/proc/stat").ok()?;
            let mut line = String::new();
            let mut reader = BufReader::new(file);

            reader.read_line(&mut line).ok()?;
            let mut parts = line.split_whitespace();
            let _cpu = parts.next()?;
            let user = parts.next()?.parse::<u64>().ok()?;
            let nice = parts.next()?.parse::<u64>().ok()?;
            let system = parts.next()?.parse::<u64>().ok()?;
            let idle = parts.next()?.parse::<u64>().ok()?;
            let iowait = parts.next()?.parse::<u64>().ok()?;
            let irq = parts.next()?.parse::<u64>().ok()?;
            let softirq = parts.next()?.parse::<u64>().ok()?;
            let steal = parts.next()?.parse::<u64>().ok()?;
            let guest = parts.next()?.parse::<u64>().ok()?;
            let guest_nice = parts.next()?.parse::<u64>().ok()?;

            let idle_all = idle + iowait;
            let total =
                user + nice + system + idle + iowait + irq + softirq + steal + guest + guest_nice;

            Some((total, idle_all))
        }

        let (total1, idle1) = match read_proc_stat() {
            Some(vals) => vals,
            None => return 0.0,
        };

        tokio::time::sleep(Duration::from_millis(100));

        let (total2, idle2) = match read_proc_stat() {
            Some(vals) => vals,
            None => return 0.0,
        };

        let total_delta = total2.saturating_sub(total1);
        let idle_delta = idle2.saturating_sub(idle1);

        if total_delta == 0 {
            0.0
        } else {
            ((total_delta - idle_delta) as f64) * 100.0 / (total_delta as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_cpu_usage;
    use std::time::Instant;

    #[tokio::test]
    async fn test_get_cpu_usage() {
        let start = Instant::now();
        let usage = get_cpu_usage().await;
        let dur = start.elapsed();
        println!("CPU usage: {:.2}%", usage);
        println!("Elapsed time: {:?}", dur);
        assert!(usage >= 0.0 && usage <= 100.0);
    }
}
