extern crate profiling_proc_macros;
pub use profiling_proc_macros::{ profile, profile_function };

use winapi::um::profileapi;
use std::mem;
use core::arch::x86_64::_rdtsc;
use profiling_proc_macros::__get_max_profile_sections;

struct ProfileSection {
    label: &'static str,
    cycles_elapsed: u64,
    hits: u64,
}

impl ProfileSection {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            cycles_elapsed: 0,
            hits: 0,
        }
    }
}

pub struct AutoProfile { section_index: usize, start_tsc: u64 }
impl AutoProfile {
    #[inline(always)]
    pub fn new(section_label: &'static str, index: usize) -> Self {
        unsafe { __GLOBAL_PROFILER.begin_section_profile(section_label, index); }
        Self { section_index: index, start_tsc: read_cpu_timer() }
    }
}
impl Drop for AutoProfile {
    // Helps guard against early returns in profile sections.
    // If an early return is triggered in a profile section, the instance of AutoProfileSection
    // will be dropped, allowing us to run this code to automatically close the profile section.
    #[inline(always)]
    fn drop(&mut self) {
        let cycles_elapsed = read_cpu_timer() - self.start_tsc;
        unsafe { __GLOBAL_PROFILER.end_section_profile(self.section_index, cycles_elapsed) }
    }
}

/// If you use any __GlobalProfiler methods directly instead of fielding its use through the
/// provided proc macros, I will find you and burn your home down.
pub struct __GlobalProfiler {
    /// The total elapsed cycles across all profile sections
    global_cycles_begin: u64,
    global_cycles_end: u64,
    sections: [Option<ProfileSection>; __get_max_profile_sections!()],
}

// FIXME Right now there is a problem with nested blocks in that we're double counting.
// Take this for example:
// profile! { "outer";
//      profile { "inner A"; ... }
//      profile { "inner B"; ... }
// }
//
// We'll count "inner A" and add its cycles to the global cycles.
// We'll count "inner B" and add its cycles to the global cycles.
// Finally we'll count "outer" and add it cycles to the global cycles.
// BUT "outer" consists of the times of both "inner A" and "inner B", so we're actually adding TOO
// MUCH to our global cycles.
//
// To fix, need to implement the idea of a scope. If we begin profiling and see we're already
// profiling another section, then we must be a nested section profile, and we should add to our
// own elapsed cycles but NOT to the global cycles.
impl __GlobalProfiler {
    const fn init() -> Self {
        Self {
            global_cycles_begin: 0,
            global_cycles_end: 0,
            sections: [ const { None }; __get_max_profile_sections!() ],
        }
    }

    #[inline(always)]
    pub fn begin_profiling(&mut self) {
        self.global_cycles_begin = read_cpu_timer();
    }

    pub fn begin_section_profile(&mut self, label: &'static str, section_index: usize) {
        // // CRITICAL
        // // This should never happen because users should not be calling global profiler methods
        // // directly. This limit should be enforced by the profile! proc macro at compile time.
        // if section_index >= __get_max_profile_sections!() {
        //     panic!(
        //         "max profile sections limit reached or attempted to index beyond sections array: {}",
        //         section_index
        //     );
        // }

        let section = match &mut self.sections[section_index] {
            Some(section) => section,
            None => {
                let section = ProfileSection::new(label);
                self.sections[section_index] = Some(section);
                self.sections[section_index].as_mut().unwrap()
            }
        };

        section.hits += 1;
    }

    pub fn end_section_profile(&mut self, section_index: usize, cycles_elapsed: u64) {
        let section = self.sections[section_index].as_mut().expect("No profile sections initialized. Was begin_profile_section called prior?");
        section.cycles_elapsed += cycles_elapsed;
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

            println!(
                "{}: {} hits, {} cycles ({:.4}%)",
                section.label,
                section.hits,
                section.cycles_elapsed,
                section.cycles_elapsed as f64 / global_cycles as f64 * 100.0
            );
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
