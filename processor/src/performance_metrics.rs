extern crate profiling_proc_macros;
pub use profiling_proc_macros::{ profile, profile_function };

use winapi::um::profileapi;
use std::mem;
use core::arch::x86_64::_rdtsc;
use profiling_proc_macros::__get_max_profile_sections;

struct ProfileSection {
    label: &'static str,
    /// Cycles of just the root profile sections (i.e. without children, recursion)
    exclusive_cycles: u64,
    /// Cycles of root profile sections with children and recursion
    inclusive_cycles: u64,
    hits: u64,
}

impl ProfileSection {
    #[cfg(feature = "profiling")]
    fn new(label: &'static str) -> Self {
        Self {
            label,
            exclusive_cycles: 0,
            inclusive_cycles: 0,
            hits: 0,
        }
    }
}

pub struct AutoProfile { section_index: usize, parent_index: Option<usize>, start_tsc: u64, root_tsc: u64 }
impl AutoProfile {
    #[cfg(feature = "profiling")]
    pub fn new(section_label: &'static str, section_index: usize) -> Self {
        let section = match unsafe { &mut __GLOBAL_PROFILER.sections[section_index] } {
            Some(section) => section,
            None => {
                let section = ProfileSection::new(section_label);
                unsafe {
                    __GLOBAL_PROFILER.sections[section_index] = Some(section);
                    __GLOBAL_PROFILER.sections[section_index].as_mut().unwrap()
                }
            }
        };

        section.hits += 1;
        let parent_index = unsafe { __GLOBAL_PROFILER.current_scope.replace(section_index) };
        Self { section_index, parent_index, start_tsc: read_cpu_timer(), root_tsc: section.inclusive_cycles }
    }
}
impl Drop for AutoProfile {
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
    sections: [Option<ProfileSection>; __get_max_profile_sections!()],
    current_scope: Option<usize>,
}

impl __GlobalProfiler {
    const fn init() -> Self {
        Self {
            global_cycles_begin: 0,
            global_cycles_end: 0,
            sections: [ const { None }; __get_max_profile_sections!() ],
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
            "Total time profiled: {:.2} ms, {} cycles (cpu freq estimate: {})",
            global_cycles as f64 / cpu_frequency as f64 * 1000.0,
            global_cycles,
            cpu_frequency,
        );

        for section in &self.sections {
            if section.is_none() { break; }
            let section = section.as_ref().unwrap();

            print!("{} [{}]: {} ({:.4}%", section.label, section.hits, section.exclusive_cycles, section.exclusive_cycles as f64 / global_cycles as f64 * 100.0);
            if section.inclusive_cycles != section.exclusive_cycles {
                print!(", {:.4}% with children", section.inclusive_cycles as f64 / global_cycles as f64 * 100.0);
            }
            println!(")");
        }
    }
}

/// DO NOT TOUCH THE GLOBAL PROFILER. USE THE PROVIDED PROC MACROS.
pub static mut __GLOBAL_PROFILER: __GlobalProfiler = __GlobalProfiler::init();

macro_rules! end_and_print_profile_info {
    ($cpu_frequency_sample_millis:expr) => {
        unsafe {
            $crate::performance_metrics::__GLOBAL_PROFILER.end_and_print_profile_info($cpu_frequency_sample_millis);
        }
    }
}
pub(crate) use end_and_print_profile_info;

macro_rules! init_profiler {
    () => {
        unsafe {
            $crate::performance_metrics::__GLOBAL_PROFILER.begin_profiling();
        }
    }
}
pub(crate) use init_profiler;

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
fn read_cpu_timer() -> u64 { unsafe { _rdtsc() } }

/// Given a sample interval in milliseconds, returns an estimate of how many CPU timer ticks occur
/// in that interval.
fn get_cpu_frequency_estimate(ms_to_wait: u64) -> u64 {
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
