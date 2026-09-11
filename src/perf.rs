//! Process performance measurement for the `--performance` booking: peak
//! resident memory and end-to-end wall time, reported in the JSON output so
//! CI can book the numbers verbatim.

/// The process's peak resident set size in bytes, where the platform
/// exposes it (Windows `GetProcessMemoryInfo`, Linux `/proc/self/status`
/// `VmHWM`). `None` where the platform does not (macOS in this build).
pub fn peak_rss_bytes() -> Option<u64> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        unsafe {
            let mut counters = PROCESS_MEMORY_COUNTERS {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                PageFaultCount: 0,
                PeakWorkingSetSize: 0,
                WorkingSetSize: 0,
                QuotaPeakPagedPoolUsage: 0,
                QuotaPagedPoolUsage: 0,
                QuotaPeakNonPagedPoolUsage: 0,
                QuotaNonPagedPoolUsage: 0,
                PagefileUsage: 0,
                PeakPagefileUsage: 0,
            };
            let ok = GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut counters,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            );
            if ok != 0 {
                Some(counters.PeakWorkingSetSize as u64)
            } else {
                None
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(value) = line.strip_prefix("VmHWM:") {
                let kib: u64 = value
                    .trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse()
                    .ok()?;
                return Some(kib * 1024);
            }
        }
        None
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}
