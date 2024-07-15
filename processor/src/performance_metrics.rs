use winapi::um::profileapi;
use std::mem;
use core::arch::x86_64::_rdtsc;

fn get_os_timer_frequency() -> u64 {
    unsafe {
        let mut freq = mem::zeroed();
        profileapi::QueryPerformanceFrequency(&mut freq);
        *freq.QuadPart() as u64
    }
}

fn read_os_timer() -> u64 {
    unsafe {
        let mut counter = mem::zeroed();
        profileapi::QueryPerformanceCounter(&mut counter);
        *counter.QuadPart() as u64
    }
}

#[inline(always)]
pub fn read_cpu_timer() -> u64 { unsafe { _rdtsc() } }

/// Given a sample interval in milliseconds, returns an estimate of how many CPU timer ticks occur
/// in that interval.
pub fn get_cpu_frequency_estimate(ms_to_wait: u64) -> u64 {
    let os_timer_frequency = get_os_timer_frequency();
    let os_wait_time = os_timer_frequency * ms_to_wait / 1000;

    let mut os_elapsed: u64 = 0;
    let cpu_start = read_cpu_timer();
    let os_start = read_os_timer();
    while os_elapsed < os_wait_time {
        os_elapsed = read_os_timer() - os_start;
    }
    let cpu_end = read_cpu_timer();
    let cpu_elapsed = cpu_end - cpu_start;

    if os_elapsed > 0 {
        os_timer_frequency * cpu_elapsed / os_elapsed
    } else {
        0
    }
}
