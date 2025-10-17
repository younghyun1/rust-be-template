pub fn get_memory_usage() -> u64 {
    #[cfg(target_os = "windows")]
    {
        use std::mem::zeroed;
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

        unsafe {
            let mut mem_status: MEMORYSTATUSEX = zeroed();
            mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
                (mem_status.ullTotalPhys - mem_status.ullAvailPhys) as u64
            } else {
                0
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use libc::{SI_LOAD_SHIFT, c_ulong, sysinfo, sysinfo as sysinfo_fn};
        use std::mem::MaybeUninit;

        unsafe {
            let mut info = MaybeUninit::<libc::sysinfo>::uninit();
            if sysinfo_fn(info.as_mut_ptr()) == 0 {
                let info = info.assume_init();
                let total = info.totalram as u64 * info.mem_unit as u64;
                let free = info.freeram as u64 * info.mem_unit as u64;
                total - free
            } else {
                0
            }
        }
    }
}
