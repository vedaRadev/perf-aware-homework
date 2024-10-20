extern crate profiling_proc_macros;
pub use profiling_proc_macros::{ profile, profile_function };

use std::{ mem, cell::OnceCell };
use winapi::{
    shared::minwindef,
    um::{ profileapi, psapi, processthreadsapi, winnt }
};
use core::arch::x86_64::_rdtsc;

#[cfg(feature = "profiling")]
use profiling_proc_macros::__get_max_profile_sections;

#[cfg(feature = "profiling")]
struct __ProfileSection {
    label: &'static str,
    /// Cycles of just the root profile sections (i.e. without children, recursion)
    exclusive_cycles: u64,
    /// Cycles of root profile sections with children and recursion
    inclusive_cycles: u64,
    bytes_processed: u64,
    hits: u64,
}

#[cfg(feature = "profiling")]
impl __ProfileSection {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            exclusive_cycles: 0,
            inclusive_cycles: 0,
            bytes_processed: 0,
            hits: 0,
        }
    }
}

#[cfg(feature = "profiling")]
pub struct __AutoProfile { section_index: usize, parent_index: Option<usize>, start_tsc: u64, root_tsc: u64 }

#[cfg(feature = "profiling")]
impl __AutoProfile {
    pub fn new(section_label: &'static str, section_index: usize, byte_count: u64) -> Self {
        let section = match unsafe { &mut __GLOBAL_PROFILER.sections[section_index] } {
            Some(section) => section,
            None => {
                let section = __ProfileSection::new(section_label);
                unsafe {
                    __GLOBAL_PROFILER.sections[section_index] = Some(section);
                    __GLOBAL_PROFILER.sections[section_index].as_mut().unwrap()
                }
            }
        };

        section.hits += 1;
        section.bytes_processed += byte_count;

        let parent_index = unsafe { __GLOBAL_PROFILER.current_scope.replace(section_index) };
        Self { section_index, parent_index, start_tsc: read_cpu_timer(), root_tsc: section.inclusive_cycles }
    }
}

#[cfg(feature = "profiling")]
impl Drop for __AutoProfile {
    // Helps guard against early returns in profile sections.
    // If an early return is triggered in a profile section, the instance of AutoProfile
    // will be dropped, allowing us to run this code to automatically close the profile section.
    fn drop(&mut self) {
        let cycles_elapsed = read_cpu_timer() - self.start_tsc;
        let section = unsafe { __GLOBAL_PROFILER.sections[self.section_index].as_mut().expect("No profile sections initialized. Was begin_profile_section called prior?") };
        // Inner nested blocks will clobber outer blocks, which is what we want here.
        section.inclusive_cycles = self.root_tsc + cycles_elapsed;
        section.exclusive_cycles = section.exclusive_cycles.wrapping_add(cycles_elapsed);
        
        unsafe { __GLOBAL_PROFILER.current_scope = self.parent_index };
        if let Some(parent_index) = self.parent_index {
            let parent_section = unsafe { __GLOBAL_PROFILER.sections[parent_index].as_mut().unwrap() };
            parent_section.exclusive_cycles = parent_section.exclusive_cycles.wrapping_sub(cycles_elapsed);
        }
    }
}

/// If you use any __GlobalProfiler methods directly instead of fielding its use through the
/// provided proc macros, I will find you and burn your home down.
pub struct __GlobalProfiler {
    global_cycles_begin: u64,
    global_cycles_end: u64,

    // We should never get an out-of-bounds error because the max profile sections invariant is
    // enforced at compile time by the proc macros, and users should NOT be interacting with the
    // global profiler except through the provided macros.
    #[cfg(feature = "profiling")]
    sections: [Option<__ProfileSection>; __get_max_profile_sections!()],

    #[cfg(feature = "profiling")]
    current_scope: Option<usize>,
}

impl __GlobalProfiler {
    const fn init() -> Self {
        Self {
            global_cycles_begin: 0,
            global_cycles_end: 0,

            #[cfg(feature = "profiling")]
            sections: [ const { None }; __get_max_profile_sections!() ],

            #[cfg(feature = "profiling")]
            current_scope: None,
        }
    }

    #[inline(always)]
    pub fn begin_profiling(&mut self) {
        self.global_cycles_begin = read_cpu_timer();
    }

    pub fn end_and_print_profile_info(&mut self, cpu_frequency_sample_millis: u64) {
        self.global_cycles_end = read_cpu_timer();
        let global_cycles = self.global_cycles_end - self.global_cycles_begin;

        let cpu_frequency = get_cpu_frequency_estimate(cpu_frequency_sample_millis);
        println!(
            "\nTotal time profiled: {:.2} ms, {} cycles (cpu freq estimate: {})",
            global_cycles as f64 / cpu_frequency as f64 * 1000.0,
            global_cycles,
            cpu_frequency,
        );

        #[cfg(feature = "profiling")]
        for section in &self.sections {
            if section.is_none() { continue; }
            let section = section.as_ref().unwrap();

            print!("\t{} [{}]: {} ({:.4}%", section.label, section.hits, section.exclusive_cycles, section.exclusive_cycles as f64 / global_cycles as f64 * 100.0);
            if section.inclusive_cycles != section.exclusive_cycles {
                print!(", {:.4}% with children", section.inclusive_cycles as f64 / global_cycles as f64 * 100.0);
            }
            print!(")");

            if section.bytes_processed > 0 {
                const MEGABYTE: u64 = 1024 * 1024;
                const GIGABYTE: u64 = MEGABYTE * 1024;

                let seconds = section.inclusive_cycles as f64 / cpu_frequency as f64;
                let bytes_per_second = section.bytes_processed as f64 / seconds;
                let megabytes = section.bytes_processed as f64 / MEGABYTE as f64;
                let gigabytes_per_second = bytes_per_second / GIGABYTE as f64;

                print!(" {:.3}mb at {:.2}gb/s", megabytes, gigabytes_per_second);
            }

            println!();
        }
    }
}

/// DO NOT TOUCH THE GLOBAL PROFILER. USE THE PROVIDED PROC MACROS.
pub static mut __GLOBAL_PROFILER: __GlobalProfiler = __GlobalProfiler::init();

#[macro_export]
macro_rules! end_and_print_profile_info {
    ($cpu_frequency_sample_millis:expr) => {
        unsafe {
            $crate::__GLOBAL_PROFILER.end_and_print_profile_info($cpu_frequency_sample_millis);
        }
    }
}

#[macro_export]
macro_rules! init_profiler {
    () => {
        unsafe {
            $crate::__GLOBAL_PROFILER.begin_profiling();
        }
    }
}

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

/// Retrieve the value of Windows' page fault counter.
/// Fun fact: this _actually_ reports the number of pages that windows has mapped, not the number
/// of page fault interrupts that have been generated, caught, or handled.
pub fn read_os_page_fault_count() -> u64 {
    static mut PROCESS_HANDLE: OnceCell<winnt::HANDLE> = OnceCell::new();

    let mut proc_mem_counters: psapi::PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    proc_mem_counters.cb = std::mem::size_of_val(&proc_mem_counters) as u32;
    unsafe {
        psapi::GetProcessMemoryInfo(
            *PROCESS_HANDLE.get_or_init(|| processthreadsapi::OpenProcess(
                winnt::PROCESS_QUERY_INFORMATION | winnt::PROCESS_VM_READ,
                minwindef::FALSE,
                processthreadsapi::GetCurrentProcessId()
            )),
            &mut proc_mem_counters,
            proc_mem_counters.cb,
        );
    }
    
    proc_mem_counters.PageFaultCount.into()
}

/// Given a sample interval in milliseconds, estimates CPU frequency in cycles per second by using
/// Windows' OS timer to measure how many CPU cycles have occurred in the time interval. Do a
/// little bit of math and you have your CPU frequency estimate. We do it this way because there is
/// no x64 intrinsic (that I know of) for getting the CPU clock rate.
pub fn get_cpu_frequency_estimate(sample_interval_millis: u64) -> u64 {
    let os_timer_frequency = get_os_timer_frequency();
    let os_wait_time = os_timer_frequency * sample_interval_millis / 1000;

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
